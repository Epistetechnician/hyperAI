import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { runInNewContext } from "node:vm";

const source = await readFile(new URL("./connection-inventory.js", import.meta.url), "utf8");
const shell = await readFile(new URL("./index.html", import.meta.url), "utf8");
const application = await readFile(new URL("./app.js", import.meta.url), "utf8");
const context = {};
runInNewContext(source, context, { filename: "connection-inventory.js" });
const { renderCards, renderList } = context.HyperAIConnectionInventoryView;

const modes = [
  "local_synthetic",
  "offline_caller_selected",
  "reference_only",
  "workspace_not_connected",
  "planned",
];

function descriptor(connection_mode, overrides = {}) {
  return {
    module_id: "sample",
    name: "Sample source",
    owner: "Sample owner",
    source_revision: "0123456789abcdef",
    connection_mode,
    state: "local_synthetic",
    boundary: "Static fixture only.",
    exposed_capability_ids: ["sample.read_only.v1"],
    next_blocked_gate: { id: "sample.authorization", summary: "Owner authorization is required." },
    ...overrides,
  };
}

function inventory(connections) {
  return {
    schema_version: "hyperai.interface.connection-inventory.v1",
    mode: "static_local_inventory",
    authority_note: "Descriptive inventory only.",
    connections,
  };
}

test("renders every connection mode and next gate as visible descriptive metadata", () => {
  const html = renderList(inventory(modes.map((mode) => descriptor(mode))));
  for (const mode of ["local synthetic", "offline caller selected", "reference only", "workspace not connected", "planned"]) {
    assert.ok(html.includes(mode), `missing mode ${mode}`);
  }
  assert.equal((html.match(/Next blocked gate/g) || []).length, modes.length);
  assert.ok(html.includes("sample.authorization"));
  assert.ok(html.includes("Owner authorization is required."));
  assert.ok(html.includes("sample.read_only.v1"));
});

test("escapes every dynamic source field and makes absent pins and capabilities explicit", () => {
  const html = renderList(inventory([descriptor("reference_only", {
    name: "<img src=x onerror='x'>",
    owner: "Owner & Co",
    source_revision: null,
    boundary: "</p><script>alert(1)</script>",
    exposed_capability_ids: [],
    next_blocked_gate: { id: "gate<unsafe>", summary: "Wait & review <later>." },
  })]));
  assert.ok(html.includes("&lt;img src=x onerror=&#39;x&#39;&gt;"));
  assert.ok(html.includes("Owner &amp; Co"));
  assert.ok(html.includes("Not pinned"));
  assert.ok(html.includes("No exposed surfaces in this interface"));
  assert.ok(html.includes("gate&lt;unsafe&gt;"));
  assert.ok(html.includes("Wait &amp; review &lt;later&gt;."));
  assert.doesNotMatch(html, /<script>alert/);
});

test("renders safe unavailable and invalid-schema states", () => {
  for (const value of [null, {}, { schema_version: "unknown", connections: [] }]) {
    const html = renderList(value);
    assert.match(html, /role="status"/);
    assert.match(html, /Connection inventory is unavailable/);
  }
});

test("overview cards use the same inventory contract and escape source text", () => {
  const html = renderCards(inventory([descriptor("planned", {
    name: "<Bazaar>",
    boundary: "Reference & rights review.",
  })]), 4);
  assert.ok(html.includes("&lt;Bazaar&gt;"));
  assert.ok(html.includes("Reference &amp; rights review."));
  assert.ok(html.includes("planned"));
  assert.match(renderCards(null), /Connection inventory is unavailable/);
});

test("links eligible local and offline entries to fixed workspace pages", () => {
  const entries = [
    ["compute_market", "local_synthetic", "market"],
    ["simple_compute_market", "offline_caller_selected", "market"],
    ["hyperintel", "offline_caller_selected", "inference"],
    ["fleet", "local_synthetic", "fleet"],
    ["eval", "local_synthetic", "eval"],
    ["trade", "local_synthetic", "trade"],
    ["data", "local_synthetic", "data"],
  ];
  const html = renderList(inventory(entries.map(([module_id, mode]) => descriptor(mode, { module_id }))));
  assert.equal((html.match(/class="connection-open"/g) || []).length, entries.length);
  for (const [, , page] of entries) {
    assert.ok(html.includes(`data-page="${page}"`), `missing fixed destination ${page}`);
  }
  assert.ok(html.includes('aria-label="Open Sample source workspace"'));
});

test("fixed destinations exist in the shell navigation and page renderer", () => {
  const entries = [
    ["compute_market", "local_synthetic", "market"],
    ["simple_compute_market", "offline_caller_selected", "market"],
    ["hyperintel", "offline_caller_selected", "inference"],
    ["fleet", "local_synthetic", "fleet"],
    ["eval", "local_synthetic", "eval"],
    ["trade", "local_synthetic", "trade"],
    ["data", "local_synthetic", "data"],
  ];
  const navigationPages = new Set(
    [...shell.matchAll(/data-page="([a-z-]+)"/g)].map((match) => match[1]),
  );
  const renderedPages = new Set(
    [...application.matchAll(/state\.page === "([a-z-]+)"/g)].map((match) => match[1]),
  );
  for (const [, , page] of entries) {
    assert.ok(navigationPages.has(page), `destination ${page} is absent from the shell`);
    assert.ok(renderedPages.has(page), `destination ${page} is absent from the page renderer`);
  }
});

test("reference-only, disconnected, planned, unknown, and spoofed entries stay informational", () => {
  const fakeCapability = ["attacker.claimed_surface.v1"];
  const entries = [
    descriptor("reference_only", { module_id: "compute_bazaar", exposed_capability_ids: fakeCapability }),
    descriptor("workspace_not_connected", { module_id: "justai", exposed_capability_ids: fakeCapability }),
    descriptor("planned", { module_id: "esn_justnet", exposed_capability_ids: fakeCapability }),
    descriptor("local_synthetic", { module_id: "fleet", exposed_capability_ids: [] }),
    descriptor("local_synthetic", { module_id: "__proto__", exposed_capability_ids: fakeCapability }),
    descriptor("local_synthetic", { module_id: "constructor", exposed_capability_ids: fakeCapability }),
  ];
  const unknownIds = Array.from({ length: 128 }, (_, index) => `future_module_${index.toString(16)}`);
  entries.push(...unknownIds.map((module_id) => descriptor("local_synthetic", {
    module_id,
    exposed_capability_ids: fakeCapability,
  })));
  const html = renderList(inventory(entries));
  assert.doesNotMatch(html, /class="connection-open"/);
  assert.ok(html.includes("attacker.claimed_surface.v1"));
});

test("workspace action labels escape source names", () => {
  const html = renderCards(inventory([descriptor("local_synthetic", {
    module_id: "data",
    name: "<Data & Tools>",
  })]));
  assert.ok(html.includes('aria-label="Open &lt;Data &amp; Tools&gt; workspace"'));
  assert.doesNotMatch(html, /aria-label="Open <Data/);
});
