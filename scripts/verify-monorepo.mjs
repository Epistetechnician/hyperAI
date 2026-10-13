import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const gates = [
  ["import manifests", root, "python3", ["scripts/verify_imports.py"]],
  ["HyperAI root", root, "corepack", ["pnpm", "run", "lint"]],
  [
    "HyperIntel nested workspace",
    path.join(root, "products/hyperintel"),
    "corepack",
    ["pnpm", "run", "verify:full"],
  ],
  [
    "JustAI nested workspace",
    path.join(root, "products/justai"),
    "corepack",
    ["pnpm", "run", "verify:full"],
  ],
  [
    "Simple Compute Market local suites",
    root,
    "node",
    ["scripts/verify-simple-market.mjs"],
  ],
];

const failures = [];
for (const [name, cwd, command, args] of gates) {
  process.stdout.write(`\n=== ${name} ===\n`);
  const result = spawnSync(command, args, {
    cwd,
    env: process.env,
    stdio: "inherit",
  });
  if (result.error) {
    process.stderr.write(`${name}: ${result.error.message}\n`);
    failures.push(name);
  } else if (result.status !== 0) {
    process.stderr.write(`${name}: exit ${result.status ?? "signal"}\n`);
    failures.push(name);
  } else {
    process.stdout.write(`PASS ${name}\n`);
  }
}

if (failures.length > 0) {
  process.stderr.write(`FAILED gates: ${failures.join(", ")}\n`);
  process.exitCode = 1;
} else {
  process.stdout.write("All configured monorepo gates passed.\n");
}
