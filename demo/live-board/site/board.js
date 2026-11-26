/* Renders live-board.json with textContent only (no HTML injection path). */
(() => {
  "use strict";
  const SCHEMA = "hyperai.demo.live-board.v1";
  const STALE_AFTER_MS = 24 * 60 * 60 * 1000;

  function validate(board) {
    return !!board && board.schema_version === SCHEMA && Array.isArray(board.rows)
      && Array.isArray(board.summary) && Array.isArray(board.nonclaims)
      && typeof board.generated_at === "string" && typeof board.board_digest === "string"
      && board.rows.every((r) => typeof r.source_url === "string" && r.source_url.startsWith("https://"));
  }

  function freshness(board, nowMs) {
    const ageMs = nowMs - Date.parse(board.generated_at);
    const stale = !Number.isFinite(ageMs) || ageMs > STALE_AFTER_MS;
    const hours = Number.isFinite(ageMs) ? Math.max(0, Math.round(ageMs / 3.6e6)) : null;
    return {
      stale,
      text: stale
        ? `Data may be stale: last export ${board.generated_at}${hours === null ? "" : ` (~${hours}h ago)`}. Prices below are historical observations.`
        : `Snapshot exported ${board.generated_at} (~${hours}h ago). Publisher observations ${board.observed_at_min} to ${board.observed_at_max}.`,
    };
  }

  function cell(doc, tag, text, className) {
    const el = doc.createElement(tag);
    el.textContent = text;
    if (className) el.className = className;
    return el;
  }

  function fill(doc, tbody, rows, build) {
    tbody.replaceChildren(...rows.map((row) => {
      const tr = doc.createElement("tr");
      build(row).forEach((c) => tr.appendChild(c));
      return tr;
    }));
  }

  function render(board, doc, nowMs) {
    const f = freshness(board, nowMs);
    const status = doc.getElementById("freshness");
    status.textContent = f.text;
    status.classList.toggle("stale", f.stale);
    fill(doc, doc.querySelector("#summary tbody"), board.summary, (s) => [
      cell(doc, "td", s.gpu_model), cell(doc, "td", s.pricing_type), cell(doc, "td", String(s.offer_count), "num"),
      cell(doc, "td", s.min_usd_per_hour, "num"), cell(doc, "td", s.median_usd_per_hour, "num"), cell(doc, "td", s.max_usd_per_hour, "num"),
    ]);
    fill(doc, doc.querySelector("#rows tbody"), board.rows, (r) => {
      const link = doc.createElement("a");
      link.href = r.source_url;
      link.rel = "noopener noreferrer";
      link.textContent = new URL(r.source_url).hostname;
      const source = doc.createElement("td");
      source.appendChild(link);
      return [cell(doc, "td", r.provider), cell(doc, "td", r.gpu_model), cell(doc, "td", r.pricing_type), cell(doc, "td", r.region),
        cell(doc, "td", r.price_usd_per_hour, "num"), cell(doc, "td", r.observed_at), source];
    });
    doc.getElementById("nonclaims").replaceChildren(...board.nonclaims.map((n) => cell(doc, "li", n)));
    doc.getElementById("ceiling").textContent = `Claim ceiling: ${board.claim_ceiling}`;
    doc.getElementById("provenance").textContent =
      `Source snapshot ${board.source.snapshot_digest}; board digest ${board.board_digest}; ${board.row_count} rows shown of ${board.source.total_feed_rows} in the feed (GPU hourly subset).`;
  }

  if (typeof module !== "undefined") module.exports = { validate, freshness, render, SCHEMA };
  if (typeof document !== "undefined") {
    fetch("live-board.json", { cache: "no-cache", credentials: "omit" })
      .then((r) => { if (!r.ok) throw new Error(String(r.status)); return r.json(); })
      .then((board) => {
        if (!validate(board)) throw new Error("unsupported board contract");
        render(board, document, Date.now());
      })
      .catch(() => {
        const status = document.getElementById("freshness");
        status.textContent = "Live board unavailable. No data is shown.";
        status.classList.add("stale");
      });
  }
})();
