#!/usr/bin/env node
/* Read-only live GPU price board exporter (state slice
 * hyperai.demo.live_price_board_static_export.v1).
 *
 * Talks only to the local owner-only HyperAI service on loopback, optionally
 * triggers its single fixed GPUDiff refresh, runs one fixed read-only query,
 * and atomically writes one static JSON file. No public listener, no
 * credentials beyond the local owner bearer file, no other network target.
 */
import { createHash, randomBytes } from "node:crypto";
import { readFile, writeFile, rename, mkdir } from "node:fs/promises";
import { join, resolve } from "node:path";
import { homedir } from "node:os";

export const BOARD_SCHEMA = "hyperai.demo.live-board.v1";
export const DATASET_ID = "hyperai.gpudiff_public_gpu_offers.v1";
export const MAX_ROWS = 500;
const DECIMAL = /^\d{1,7}(\.\d{1,8})?$/;
const NONCLAIMS = [
  "Aggregator-published public list prices; not direct provider connections.",
  "Not an executable offer, quote, reservation, or current capacity.",
  "No purchase, order, settlement, or provider call is possible from this site.",
  "Not a forecast, return, benchmark, or production-readiness claim.",
  "Price records are publisher-reported and not independently verified.",
];

export const QUERY_SQL =
  "SELECT provider, gpu_model, price_decimal, pricing_type, region, vram_gb, observed_at, provenance_url " +
  "FROM dataset WHERE unit = 'usd_per_hour' AND gpu_model IS NOT NULL " +
  "ORDER BY gpu_model, pricing_type, CAST(price_decimal AS DOUBLE)";

export function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((k) => `${JSON.stringify(k)}:${canonicalJson(value[k])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

export function sha256Hex(text) {
  return createHash("sha256").update(text).digest("hex");
}

function micros(decimal) {
  const [whole, frac = ""] = decimal.split(".");
  return Number(whole) * 1e8 + Number((frac + "00000000").slice(0, 8));
}

function isHttps(url) {
  try {
    const parsed = new URL(url);
    return parsed.protocol === "https:" && !parsed.username && !parsed.password;
  } catch {
    return false;
  }
}

function clean(value, max) {
  if (value == null) return null;
  const text = String(value);
  if (text.length === 0 || text.length > max || /[\u0000-\u001f\u007f]/.test(text)) return null;
  return text;
}

/** Validates and normalizes query rows; rejects (drops) anything malformed. */
export function normalizeRows(columns, rows) {
  const index = Object.fromEntries(columns.map((name, i) => [name, i]));
  for (const required of ["provider", "gpu_model", "price_decimal", "pricing_type", "region", "observed_at", "provenance_url"]) {
    if (!(required in index)) throw new Error(`query result missing column ${required}`);
  }
  const out = [];
  let dropped = 0;
  for (const row of rows) {
    const price = row[index.price_decimal];
    const url = row[index.provenance_url];
    const item = {
      provider: clean(row[index.provider], 64),
      gpu_model: clean(row[index.gpu_model], 96),
      price_usd_per_hour: typeof price === "string" && DECIMAL.test(price) ? price : null,
      pricing_type: clean(row[index.pricing_type], 32),
      region: clean(row[index.region], 64),
      vram_gb: "vram_gb" in index ? clean(row[index.vram_gb], 16) : null,
      observed_at: clean(row[index.observed_at], 40),
      source_url: typeof url === "string" && isHttps(url) ? url : null,
    };
    if (!item.provider || !item.gpu_model || !item.price_usd_per_hour || !item.pricing_type
      || !item.region || !item.observed_at || !item.source_url) {
      dropped += 1;
      continue;
    }
    out.push(item);
  }
  return { rows: out, dropped };
}

/** Per-model min/median/max over on-demand and spot separately (integer math). */
export function summarize(rows) {
  const groups = new Map();
  for (const row of rows) {
    const key = `${row.gpu_model}\u0000${row.pricing_type}`;
    if (!groups.has(key)) groups.set(key, { gpu_model: row.gpu_model, pricing_type: row.pricing_type, values: [] });
    groups.get(key).values.push(row);
  }
  return [...groups.values()].map((group) => {
    const sorted = group.values.slice().sort((a, b) => micros(a.price_usd_per_hour) - micros(b.price_usd_per_hour));
    const mid = Math.floor((sorted.length - 1) / 2);
    return {
      gpu_model: group.gpu_model,
      pricing_type: group.pricing_type,
      offer_count: sorted.length,
      min_usd_per_hour: sorted[0].price_usd_per_hour,
      median_usd_per_hour: sorted[mid].price_usd_per_hour,
      max_usd_per_hour: sorted[sorted.length - 1].price_usd_per_hour,
    };
  }).sort((a, b) => a.gpu_model.localeCompare(b.gpu_model) || a.pricing_type.localeCompare(b.pricing_type));
}

export function buildBoard({ catalogEntry, queryResult, now = new Date() }) {
  if (!catalogEntry || catalogEntry.dataset_id !== DATASET_ID) throw new Error("catalog entry for GPUDiff dataset missing");
  if (queryResult.dataset_id !== DATASET_ID || queryResult.snapshot_digest !== catalogEntry.snapshot_digest) {
    throw new Error("query snapshot digest does not match catalog");
  }
  const { rows, dropped } = normalizeRows(queryResult.columns, queryResult.rows);
  if (rows.length === 0) throw new Error("no valid GPU hourly rows");
  const observed = rows.map((r) => r.observed_at).sort();
  const body = {
    schema_version: BOARD_SCHEMA,
    generated_at: now.toISOString(),
    source: {
      label: catalogEntry.source_label,
      license_note: "CC BY 4.0; cite gpudiff.com",
      url: "https://gpudiff.com/api/v1/offers.json",
      snapshot_digest: catalogEntry.snapshot_digest,
      total_feed_rows: catalogEntry.row_count,
    },
    claim_ceiling: catalogEntry.claim_ceiling,
    nonclaims: NONCLAIMS,
    observed_at_min: observed[0],
    observed_at_max: observed[observed.length - 1],
    row_count: rows.length,
    dropped_row_count: dropped,
    summary: summarize(rows),
    rows,
  };
  return { ...body, board_digest: sha256Hex(canonicalJson(body)) };
}

async function readJson(response) {
  if (!response.ok) throw new Error(`service returned HTTP ${response.status}`);
  return response.json();
}

export async function run({ home, refresh, outDir, fetchImpl = fetch }) {
  const runtime = join(home, "runtime");
  const record = JSON.parse(await readFile(join(runtime, "service.json"), "utf8"));
  const token = (await readFile(join(runtime, "bearer.token"), "utf8")).trim();
  const address = String(record.address);
  if (!/^127\.0\.0\.1:\d{2,5}$/.test(address)) throw new Error("service address is not IPv4 loopback");
  const base = `http://${address}`;
  const headers = { Authorization: `Bearer ${token}` };
  const post = (path, body) => fetchImpl(`${base}${path}`, {
    method: "POST",
    headers: { ...headers, "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });

  if (refresh) await readJson(await post("/api/v1/data/gpudiff/refresh", {}));
  const catalog = await readJson(await fetchImpl(`${base}/api/v1/data/catalog`, { headers }));
  const catalogEntry = (catalog.datasets || []).find((d) => d.dataset_id === DATASET_ID);
  const queryResult = await readJson(await post("/api/v1/data/query", {
    query_id: `board-${Date.now()}-${randomBytes(4).toString("hex")}`,
    request: { dataset_id: DATASET_ID, sql: QUERY_SQL, format: "json", max_rows: MAX_ROWS },
  }));
  const board = buildBoard({ catalogEntry, queryResult });

  await mkdir(outDir, { recursive: true });
  const target = join(outDir, "live-board.json");
  const temporary = join(outDir, `.live-board.${randomBytes(6).toString("hex")}.tmp`);
  await writeFile(temporary, `${JSON.stringify(board, null, 2)}\n`, { mode: 0o644 });
  await rename(temporary, target);
  return { target, board };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const args = process.argv.slice(2);
  const flag = (name) => args.includes(name);
  const value = (name, fallback) => {
    const i = args.indexOf(name);
    return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
  };
  run({
    home: resolve(value("--home", process.env.HYPERAI_HOME || join(homedir(), ".hyperai"))),
    refresh: flag("--refresh"),
    outDir: resolve(value("--out", "demo/live-board/site")),
  }).then(({ target, board }) => {
    console.log(`wrote ${target} rows=${board.row_count} digest=${board.board_digest}`);
  }).catch((error) => {
    console.error(`export failed: ${error.message}`);
    process.exit(1);
  });
}
