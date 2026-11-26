import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, readFile, readdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { buildBoard, canonicalJson, normalizeRows, run, summarize, DATASET_ID, QUERY_SQL } from "./export.mjs";

const columns = ["provider", "gpu_model", "price_decimal", "pricing_type", "region", "vram_gb", "observed_at", "provenance_url"];
const good = (over = []) => {
  const row = ["vast", "H200", "3.50", "on_demand", "global", "141", "2026-10-20T19:14:30Z", "https://vast.ai/pricing"];
  over.forEach(([i, v]) => { row[i] = v; });
  return row;
};
const entry = { dataset_id: DATASET_ID, snapshot_digest: "a".repeat(64), source_label: "GPUDiff; CC BY 4.0", row_count: 2581, claim_ceiling: "ceiling" };

test("normalizeRows drops malformed rows and non-https sources", () => {
  const { rows, dropped } = normalizeRows(columns, [
    good(),
    good([[2, "free"]]),
    good([[7, "http://insecure.example/x"]]),
    good([[7, "https://user:pw@evil.example/x"]]),
    good([[1, "bad\u0007model"]]),
    good([[2, "-1"]]),
  ]);
  assert.equal(rows.length, 1);
  assert.equal(dropped, 5);
});

test("summarize uses exact decimal ordering and lower median", () => {
  const rows = ["10.00", "9.5", "2.25", "100.00"].map((p) => ({ gpu_model: "X", pricing_type: "spot", price_usd_per_hour: p }));
  const [s] = summarize(rows);
  assert.deepEqual([s.min_usd_per_hour, s.median_usd_per_hour, s.max_usd_per_hour, s.offer_count], ["2.25", "9.5", "100.00", 4]);
});

test("buildBoard binds snapshot digest and is deterministic apart from time", () => {
  const queryResult = { dataset_id: DATASET_ID, snapshot_digest: entry.snapshot_digest, columns, rows: [good(), good([[0, "runpod"]])] };
  const now = new Date("2026-10-21T00:00:00Z");
  const a = buildBoard({ catalogEntry: entry, queryResult, now });
  const b = buildBoard({ catalogEntry: entry, queryResult, now });
  assert.equal(a.board_digest, b.board_digest);
  assert.equal(a.claim_ceiling, "ceiling");
  assert.throws(() => buildBoard({ catalogEntry: entry, queryResult: { ...queryResult, snapshot_digest: "b".repeat(64) }, now }), /does not match/);
  assert.throws(() => buildBoard({ catalogEntry: entry, queryResult: { ...queryResult, rows: [] }, now }), /no valid/);
});

test("canonicalJson sorts keys", () => {
  assert.equal(canonicalJson({ b: 1, a: [{ d: 1, c: 2 }] }), '{"a":[{"c":2,"d":1}],"b":1}');
});

test("run talks only to loopback with bearer, refreshes on request, writes atomically", async () => {
  const home = await mkdtemp(join(tmpdir(), "board-home-"));
  await mkdir(join(home, "runtime"));
  await writeFile(join(home, "runtime", "service.json"), JSON.stringify({ address: "127.0.0.1:4317" }));
  await writeFile(join(home, "runtime", "bearer.token"), "t".repeat(48));
  const out = await mkdtemp(join(tmpdir(), "board-out-"));
  const calls = [];
  const fetchImpl = async (url, init = {}) => {
    calls.push({ url, method: init.method || "GET", auth: init.headers?.Authorization });
    const json = (value) => ({ ok: true, status: 200, json: async () => value });
    if (url.endsWith("/refresh")) return json({});
    if (url.endsWith("/catalog")) return json({ datasets: [entry] });
    const body = JSON.parse(init.body);
    assert.equal(body.request.sql, QUERY_SQL);
    assert.match(body.query_id, /^board-/);
    return json({ dataset_id: DATASET_ID, snapshot_digest: entry.snapshot_digest, columns, rows: [good()] });
  };
  const { target, board } = await run({ home, refresh: true, outDir: out, fetchImpl });
  assert.ok(calls.every((c) => c.url.startsWith("http://127.0.0.1:4317/") && c.auth === `Bearer ${"t".repeat(48)}`));
  assert.equal(calls[0].method, "POST");
  assert.equal(JSON.parse(await readFile(target, "utf8")).board_digest, board.board_digest);
  assert.deepEqual((await readdir(out)).sort(), ["live-board.json"]);
});

test("run refuses a non-loopback service record", async () => {
  const home = await mkdtemp(join(tmpdir(), "board-home-"));
  await mkdir(join(home, "runtime"));
  await writeFile(join(home, "runtime", "service.json"), JSON.stringify({ address: "10.0.0.5:4317" }));
  await writeFile(join(home, "runtime", "bearer.token"), "t".repeat(48));
  await assert.rejects(run({ home, refresh: false, outDir: tmpdir(), fetchImpl: async () => { throw new Error("must not fetch"); } }), /loopback/);
});
