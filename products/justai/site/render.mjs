#!/usr/bin/env node

import fs from "node:fs";
import { createHash, createPublicKey, verify as verifySignature } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";

const DIGEST = /^[0-9a-f]{64}$/;
const BRIDGE_SCHEMAS = new Set([
  "justai.artifact_manifest.v1",
  "justai.evaluation_receipt.v1",
  "justai.disclosure_decision.v1",
  "justai.claim_projection.v1",
  "justai.policy_trace.v1",
]);
const FORBIDDEN_DISCLOSURE_FIELDS = new Set([
  "raw_payload", "raw_bytes", "private_locator", "secret", "credential", "restricted_payload",
]);
const ED25519_SPKI_PREFIX = Buffer.from("302a300506032b6570032100", "hex");

function usage() {
  console.error("Usage: node site/render.mjs --release release.json --out output.html");
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function renderDetail(value) {
  if (Array.isArray(value)) {
    return `<ul>${value.map((item) => `<li>${escapeHtml(JSON.stringify(item))}</li>`).join("")}</ul>`;
  }
  return `<code>${escapeHtml(JSON.stringify(value))}</code>`;
}

function canonicalize(value) {
  if (value === null) return "null";
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "string") return JSON.stringify(value);
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new Error("non-finite JSON number");
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) return `[${value.map(canonicalize).join(",")}]`;
  if (typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalize(value[key])}`).join(",")}}`;
  }
  throw new Error("unsupported JSON value");
}

export function contentDigest(value) {
  const content = Object.fromEntries(Object.entries(value).filter(([key]) => key !== "content_digest" && key !== "signature_refs"));
  return createHash("sha256").update(canonicalize(content), "utf8").digest("hex");
}

export function signaturePayload(schema, digest, purpose) {
  return Buffer.from(`justai:${schema}:${digest}:${purpose}`, "utf8");
}

function verifyReleaseSignatures(release) {
  for (const reference of release.signature_refs) {
    if (reference.purpose !== "release") throw new Error(`unsupported signature purpose: ${reference.purpose}`);
    if (typeof reference.public_key !== "string" || !/^[0-9a-f]{64}$/.test(reference.public_key)) {
      throw new Error(`invalid public key for ${reference.key_id}`);
    }
    if (typeof reference.signature !== "string" || !/^[0-9a-f]{128}$/.test(reference.signature)) {
      throw new Error(`invalid signature for ${reference.key_id}`);
    }
    const publicKey = createPublicKey({
      key: Buffer.concat([ED25519_SPKI_PREFIX, Buffer.from(reference.public_key, "hex")]),
      format: "der",
      type: "spki",
    });
    const valid = verifySignature(
      null,
      signaturePayload(release.schema, release.content_digest, reference.purpose),
      publicKey,
      Buffer.from(reference.signature, "hex"),
    );
    if (!valid) throw new Error(`signature verification failed for ${reference.key_id}`);
  }
}

function verifyRecordSignatures(record) {
  if (!Array.isArray(record.signature_refs) || record.signature_refs.length === 0) {
    throw new Error(`record ${record.record_id} requires a signature`);
  }
  for (const reference of record.signature_refs) {
    if (typeof reference.purpose !== "string" || typeof reference.key_id !== "string") {
      throw new Error(`invalid signature reference for ${record.record_id}`);
    }
    if (typeof reference.public_key !== "string" || !/^[0-9a-f]{64}$/.test(reference.public_key)) {
      throw new Error(`invalid public key for ${reference.key_id}`);
    }
    if (typeof reference.signature !== "string" || !/^[0-9a-f]{128}$/.test(reference.signature)) {
      throw new Error(`invalid signature for ${reference.key_id}`);
    }
    const publicKey = createPublicKey({
      key: Buffer.concat([ED25519_SPKI_PREFIX, Buffer.from(reference.public_key, "hex")]),
      format: "der",
      type: "spki",
    });
    if (!verifySignature(
      null,
      signaturePayload(record.schema, record.content_digest, reference.purpose),
      publicKey,
      Buffer.from(reference.signature, "hex"),
    )) throw new Error(`signature verification failed for ${reference.key_id}`);
  }
}

function rejectDisclosureFields(value) {
  if (Array.isArray(value)) {
    for (const item of value) rejectDisclosureFields(item);
  } else if (value && typeof value === "object") {
    for (const [key, item] of Object.entries(value)) {
      if (FORBIDDEN_DISCLOSURE_FIELDS.has(key)) throw new Error(`public release contains forbidden disclosure field: ${key}`);
      rejectDisclosureFields(item);
    }
  }
}

function assertString(object, key) {
  if (typeof object[key] !== "string" || object[key].length === 0) {
    throw new Error(`${key} must be a non-empty string`);
  }
}

export function validateRelease(release) {
  if (!release || typeof release !== "object" || Array.isArray(release)) {
    throw new Error("release must be an object");
  }
  if (release.schema !== "justai.release.v1") {
    throw new Error("unsupported release schema");
  }
  for (const key of [
    "record_id",
    "state_slice",
    "status",
    "claim_ceiling",
    "created_at",
    "publisher_id",
    "content_digest",
  ]) assertString(release, key);
  if (!DIGEST.test(release.content_digest)) throw new Error("invalid release digest");
  if (contentDigest(release) !== release.content_digest) throw new Error("release content digest mismatch");
  if (!Array.isArray(release.records)) throw new Error("records must be an array");
  rejectDisclosureFields(release.records);
  if (!Array.isArray(release.signature_refs) || release.signature_refs.length === 0) {
    throw new Error("release requires a signature reference");
  }
  verifyReleaseSignatures(release);
  const ids = new Set();
  for (const record of release.records) {
    if (!record || typeof record !== "object" || Array.isArray(record)) throw new Error("record must be an object");
    for (const key of ["schema", "record_id", "status", "claim_ceiling", "content_digest"]) assertString(record, key);
    if (BRIDGE_SCHEMAS.has(record.schema) && record.state_slice !== "justai-evidence-bridge-v1") throw new Error(`bridge record ${record.record_id} has the wrong state slice`);
    if (!DIGEST.test(record.content_digest)) throw new Error(`invalid digest for ${record.record_id}`);
    if (contentDigest(record) !== record.content_digest) throw new Error(`content digest mismatch for ${record.record_id}`);
    if (ids.has(record.record_id)) throw new Error(`duplicate record_id: ${record.record_id}`);
    ids.add(record.record_id);
  }
  const hasBridgeRecords = release.records.some((record) => BRIDGE_SCHEMAS.has(record.schema));
  if (hasBridgeRecords) validateBridgeReferences(release);
  return release;
}

function validateBridgeReferences(release) {
  const recordsById = new Map(release.records.map((record) => [record.record_id, record]));
  const knownDigests = new Set(release.records.map((record) => record.content_digest));
  const reviewerManifests = new Map();
  const reviewsByTarget = new Map();
  for (const record of release.records) {
    if (record.schema === "justai.source.v1") {
      for (const digest of record.source_digests ?? []) knownDigests.add(digest);
    }
    if (record.schema === "justai.artifact_manifest.v1") knownDigests.add(record.artifact_digest);
    if (record.schema === "justai.syndicate.v1") reviewerManifests.set(record.record_id, record);
    if (record.schema === "justai.review_receipt.v1") {
      const reviews = reviewsByTarget.get(record.reviewed_record_id) ?? [];
      reviews.push(record);
      reviewsByTarget.set(record.reviewed_record_id, reviews);
    }
  }
  for (const record of release.records) {
    if (record.schema === "justai.review_receipt.v1") {
      const target = recordsById.get(record.reviewed_record_id);
      if (!target || target.content_digest !== record.reviewed_digest) {
        throw new Error(`review digest does not match ${record.reviewed_record_id}`);
      }
      const manifest = reviewerManifests.get(record.syndicate_manifest_ref);
      if (!manifest || manifest.syndicate_id !== record.reviewer_org) {
        throw new Error(`reviewer manifest is not bound for ${record.record_id}`);
      }
      verifyRecordSignatures(manifest);
    }
    const core = new Set([
      "justai.claim.v1", "justai.incident.v1", "justai.commitment.v1",
      "justai.evaluation_receipt.v1", "justai.disclosure_decision.v1",
      "justai.claim_projection.v1", "justai.policy_trace.v1",
    ]);
    if (core.has(record.schema)) {
      if (!Array.isArray(record.source_digests) || record.source_digests.length === 0 || record.source_digests.some((digest) => !knownDigests.has(digest))) {
        throw new Error(`unresolved source provenance for ${record.record_id}`);
      }
      const refs = record.review_receipt_refs ?? [];
      const reviews = reviewsByTarget.get(record.record_id) ?? [];
      const accepted = refs.map((id) => reviews.find((review) => review.record_id === id));
      if (refs.length < 2 || accepted.some((review) => !review || review.result !== "accept" || review.conflict_of_interest || review.reviewer_org === release.publisher_id) || new Set(accepted.map((review) => review.reviewer_org)).size < 2) {
        throw new Error(`invalid independent reviews for ${record.record_id}`);
      }
    }
    for (const key of ["support_refs", "contradiction_refs", "evidence_refs", "subject_refs", "artifact_manifest_refs", "disclosure_decision_refs", "claim_projection_refs", "evaluation_receipt_refs", "artifact_refs"]) {
      for (const reference of record[key] ?? []) {
        if (!recordsById.has(reference)) throw new Error(`unknown record reference ${reference} in ${record.record_id}`);
      }
    }
    const requireSchema = (reference, schema) => {
      if (recordsById.get(reference).schema !== schema) throw new Error(`${record.record_id} reference ${reference} must use ${schema}`);
    };
    if (record.schema === "justai.evaluation_receipt.v1") {
      for (const reference of record.artifact_manifest_refs) {
        requireSchema(reference, "justai.artifact_manifest.v1");
        if (!record.source_digests.includes(recordsById.get(reference).artifact_digest)) throw new Error(`evaluation ${record.record_id} does not bind artifact digest`);
      }
      for (const reference of record.disclosure_decision_refs) requireSchema(reference, "justai.disclosure_decision.v1");
    }
    if (record.schema === "justai.claim_projection.v1") {
      for (const reference of record.disclosure_decision_refs) requireSchema(reference, "justai.disclosure_decision.v1");
    }
    if (record.schema === "justai.policy_trace.v1") {
      for (const reference of record.artifact_refs) {
        requireSchema(reference, "justai.artifact_manifest.v1");
        if (!record.source_digests.includes(recordsById.get(reference).artifact_digest)) throw new Error(`policy trace ${record.record_id} does not bind artifact digest`);
      }
      for (const reference of record.evaluation_receipt_refs) requireSchema(reference, "justai.evaluation_receipt.v1");
      for (const reference of record.claim_projection_refs) requireSchema(reference, "justai.claim_projection.v1");
    }
    if (record.schema === "justai.dispute.v1" && !recordsById.has(record.disputed_record_id)) {
      throw new Error(`unknown disputed record ${record.disputed_record_id}`);
    }
  }
}

function displayValue(record, key, fallback = "Not declared") {
  const value = record[key];
  if (Array.isArray(value)) return value.length ? value.join("; ") : fallback;
  return value === undefined || value === "" ? fallback : value;
}

export function render(release) {
  const records = release.records.map((record) => {
    const details = Object.entries(record)
      .filter(([key]) => ![
        "schema", "record_id", "state_slice", "status", "claim_ceiling", "created_at",
        "publisher_id", "content_digest", "signature_refs"
      ].includes(key))
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, value]) => `<dt>${escapeHtml(key)}</dt><dd>${renderDetail(value)}</dd>`)
      .join("");
    return `
    <article class="record" id="${escapeHtml(record.record_id)}">
      <h2>${escapeHtml(record.record_id)}</h2>
      <dl>
        <dt>Schema</dt><dd>${escapeHtml(record.schema)}</dd>
        <dt>Status</dt><dd>${escapeHtml(record.status)}</dd>
        <dt>Claim ceiling</dt><dd>${escapeHtml(record.claim_ceiling)}</dd>
        <dt>Digest</dt><dd><code>${escapeHtml(record.content_digest)}</code></dd>
        <dt>Evidence basis</dt><dd>${escapeHtml(displayValue(record, "evidence_basis", displayValue(record, "custody_mode", "Not declared")))}</dd>
        <dt>Uncertainty</dt><dd>${escapeHtml(displayValue(record, "uncertainty"))}</dd>
        <dt>Missing information</dt><dd>${escapeHtml(displayValue(record, "missing_information"))}</dd>
        <dt>Contestation</dt><dd>${escapeHtml(displayValue(record, "contestation", displayValue(record, "affected_party_response")))}</dd>
        <dt>Withheld material</dt><dd>${escapeHtml(displayValue(record, "withheld_material", record.access_class && record.access_class !== "public" ? `Access class: ${record.access_class}` : "None declared"))}</dd>
        ${details}
      </dl>
    </article>`;
  }).join("\n");
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="description" content="Signed JustAI public observatory release">
  <title>JustAI public observatory</title>
  <style>
    :root { color-scheme: light; font-family: system-ui, sans-serif; }
    body { max-width: 70rem; margin: 0 auto; padding: 2rem; color: #18212b; background: #f6f8fa; }
    header, .record { background: white; border: 1px solid #d8dee4; border-radius: .75rem; padding: 1.25rem; margin-bottom: 1rem; }
    .meta { color: #57606a; }
    code { overflow-wrap: anywhere; }
  </style>
</head>
<body>
  <header>
    <p class="meta">Signed release: ${escapeHtml(release.record_id)}</p>
    <h1>JustAI public observatory</h1>
    <p>${escapeHtml(release.records.length)} records · status ${escapeHtml(release.status)} · claim ceiling ${escapeHtml(release.claim_ceiling)}</p>
    <p class="meta">Publisher ${escapeHtml(release.publisher_id)} · digest <code>${escapeHtml(release.content_digest)}</code></p>
  </header>
  ${records}
</body>
</html>
`;
}

function main(argv) {
  const releaseIndex = argv.indexOf("--release");
  const outputIndex = argv.indexOf("--out");
  if (argv.includes("--help")) { usage(); return 0; }
  if (releaseIndex < 0 || outputIndex < 0 || !argv[releaseIndex + 1] || !argv[outputIndex + 1]) {
    usage(); return 2;
  }
  try {
    const release = validateRelease(JSON.parse(fs.readFileSync(path.resolve(argv[releaseIndex + 1]), "utf8")));
    fs.mkdirSync(path.dirname(path.resolve(argv[outputIndex + 1])), { recursive: true });
    fs.writeFileSync(path.resolve(argv[outputIndex + 1]), render(release), "utf8");
    return 0;
  } catch (error) {
    console.error(`error: ${error.message}`);
    return 1;
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = main(process.argv.slice(2));
}
