import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { runInNewContext } from "node:vm";

const source = await readFile(new URL("./error-envelope.js", import.meta.url), "utf8");
const context = {};
runInNewContext(source, context);
const normalize = context.HyperAIErrorEnvelope.normalize;

test("keeps actual HTTP status and a valid versioned service code", () => {
  const result = normalize(422, {
    schema_version: "hyperai.interface.error.v1",
    status: "error",
    code: "DATA_QUERY_REJECTED",
    message: "request details must stay local",
  });
  assert.equal(result.status, 422);
  assert.equal(result.code, "DATA_QUERY_REJECTED");
  assert.equal(result.message, "The local HyperAI contract operation could not complete.");
});

test("maps the stale supply-comparison conflict to fixed recovery copy only", () => {
  const rawMessage = "untrusted service detail must not be rendered";
  const result = normalize(409, {
    schema_version: "hyperai.interface.error.v1",
    status: "error",
    code: "MARKET_SUPPLY_COMPARISON_CHANGED",
    message: rawMessage,
  });
  assert.equal(result.status, 409);
  assert.equal(result.code, "MARKET_SUPPLY_COMPARISON_CHANGED");
  assert.equal(
    result.message,
    "This supply comparison is out of date. Reload this page to get a fresh comparison before requesting a quote.",
  );
  assert.ok(!result.message.includes(rawMessage));

  for (const code of ["MARKET_SUPPLY_FUTURE_CONFLICT", "constructor", "toString"]) {
    const unknown = normalize(409, {
      schema_version: "hyperai.interface.error.v1",
      status: "error",
      code,
      message: rawMessage,
    });
    assert.equal(unknown.message, "The local HyperAI contract operation could not complete.");
  }
});

test("normalizes malformed, unknown-version, and invalid-status envelopes", () => {
  for (const body of [
    null,
    [],
    { schema_version: "other.v1", status: "error", code: "DATA_QUERY_REJECTED" },
    { schema_version: "hyperai.interface.error.v1", status: "ok", code: "DATA_QUERY_REJECTED" },
    { schema_version: "hyperai.interface.error.v1", status: "error", code: "BAD\nCODE" },
    { schema_version: "hyperai.interface.error.v1", status: "error", code: "C".repeat(81) },
  ]) {
    const result = normalize(500, body);
    assert.equal(result.status, 500);
    assert.equal(result.code, "UNKNOWN_ERROR");
    assert.equal(result.message, "The local HyperAI contract operation could not complete.");
  }
});

test("does not display response-body messages or accept an invalid HTTP status", () => {
  const result = normalize(0, {
    schema_version: "hyperai.interface.error.v1",
    status: "error",
    code: "UNAUTHENTICATED",
    message: `${"private".repeat(100)}\u001b`,
  });
  assert.equal(result.status, 0);
  assert.equal(result.code, "UNAUTHENTICATED");
  assert.equal(result.message, "The local HyperAI contract operation could not complete.");
});
