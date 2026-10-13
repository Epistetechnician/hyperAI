import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { spawnSync } from "node:child_process";
import { generateKeyPairSync, sign } from "node:crypto";
import { contentDigest, signaturePayload, validateRelease } from "./render.mjs";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const renderer = path.join(root, "site", "render.mjs");

function release(records = []) {
  const { privateKey, publicKey } = generateKeyPairSync("ed25519");
  const publicKeyHex = publicKey.export({ type: "spki", format: "der" }).subarray(-32).toString("hex");
  const value = {
    schema: "justai.release.v1",
    record_id: "release-test",
    state_slice: "justai-public-observatory-v1",
    status: "Published",
    claim_ceiling: "bounded-description-only",
    created_at: "2026-09-09T12:00:00Z",
    publisher_id: "syndicate-test",
    source_digests: [],
    content_digest: "",
    signature_refs: [],
    records,
    supersedes: [],
  };
  value.content_digest = contentDigest(value);
  value.signature_refs = [{
    key_id: "fixture-key",
    purpose: "release",
    public_key: publicKeyHex,
    signature: sign(null, signaturePayload(value.schema, value.content_digest, "release"), privateKey).toString("hex"),
  }];
  return value;
}

test("renders escaped, deterministic public records", () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "justai-render-"));
  const input = path.join(directory, "release.json");
  const output = path.join(directory, "site", "index.html");
  const record = {
    schema: "justai.claim.v1",
    record_id: "claim-<safe>",
    state_slice: "justai-observatory-contracts-v1",
    status: "Contested",
    claim_ceiling: "bounded-description-only",
    created_at: "2026-09-09T12:00:00Z",
    publisher_id: "syndicate-test",
    source_digests: [],
    content_digest: "",
    signature_refs: [],
  };
  record.content_digest = contentDigest(record);
  fs.writeFileSync(input, JSON.stringify(release([record])));
  const result = spawnSync(process.execPath, [renderer, "--release", input, "--out", output], { encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr);
  const html = fs.readFileSync(output, "utf8");
  assert.match(html, /claim-&lt;safe&gt;/);
  assert.doesNotMatch(html, /<h2>claim-<safe><\/h2>/);
});

test("rejects unsigned releases", () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "justai-render-"));
  const input = path.join(directory, "release.json");
  const output = path.join(directory, "index.html");
  const value = release();
  value.signature_refs = [];
  fs.writeFileSync(input, JSON.stringify(value));
  const result = spawnSync(process.execPath, [renderer, "--release", input, "--out", output], { encoding: "utf8" });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /requires a signature reference/);
});

test("rejects altered release content even when signature metadata remains", () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "justai-render-"));
  const input = path.join(directory, "release.json");
  const output = path.join(directory, "index.html");
  const value = release();
  value.records = [{ ...value.records[0], status: "Withdrawn" }];
  fs.writeFileSync(input, JSON.stringify(value));
  const result = spawnSync(process.execPath, [renderer, "--release", input, "--out", output], { encoding: "utf8" });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /content digest mismatch/);
});

test("rejects disclosure fields before rendering a public receipt", () => {
  const artifact = {
    schema: "justai.artifact_manifest.v1",
    record_id: "artifact-leak",
    state_slice: "justai-evidence-bridge-v1",
    status: "Unknown",
    claim_ceiling: "bounded-observation-only",
    created_at: "2026-09-09T12:00:00Z",
    publisher_id: "syndicate-test",
    source_digests: [],
    content_digest: "",
    signature_refs: [],
    artifact_kind: "trace",
    access_class: "restricted",
    custody_mode: "unknown",
    license_basis: "synthetic",
    custodian_id: "custodian-test",
    acquisition_timestamp: "2026-09-09T12:00:00Z",
    artifact_digest: "1".repeat(64),
    permitted_uses: ["bounded-evaluation"],
    retention_and_disclosure_rules: "withhold raw material",
    evidence_basis: "unknown",
  };
  artifact.content_digest = contentDigest(artifact);
  const value = release([artifact]);
  value.records[0].raw_payload = "restricted bytes";
  value.records[0].content_digest = contentDigest(value.records[0]);
  value.content_digest = contentDigest(value);
  assert.throws(() => validateRelease(value), /forbidden disclosure field/);
});
