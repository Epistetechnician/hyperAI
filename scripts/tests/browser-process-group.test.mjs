import assert from "node:assert/strict";
import test from "node:test";

import {
  hasLiveProcessGroupMembers,
  parseProcessGroupMembers,
} from "../browser-process-group.mjs";

test("process-group parser retains members for the requested group only", () => {
  const members = parseProcessGroupMembers([
    "  700  700  S+",
    "  701  700  Z+",
    "  800  800  R",
  ].join("\n"), 700);

  assert.deepEqual(members, [
    { pid: 700, processGroupId: 700, state: "S+" },
    { pid: 701, processGroupId: 700, state: "Z+" },
  ]);
});

test("a group containing only defunct members is execution-empty", () => {
  const members = parseProcessGroupMembers("  701  700  Z+", 700);

  assert.equal(hasLiveProcessGroupMembers(members), false);
});

test("live, stopped, and unknown process states remain cleanup-active", () => {
  for (const state of ["S", "T+", "?"]) {
    const members = parseProcessGroupMembers(`  701  700  ${state}`, 700);
    assert.equal(hasLiveProcessGroupMembers(members), true, state);
  }
});

test("malformed rows cannot create a matching process-group member", () => {
  const output = [
    "PID PGID STAT",
    "  invalid  700  S",
    "  701  invalid  S",
    "  702  700",
    "  703  800  R",
    "  704  700  S extra",
  ].join("\n");

  assert.deepEqual(parseProcessGroupMembers(output, 700), []);
  assert.deepEqual(parseProcessGroupMembers("  701  700  S", 0), []);
});
