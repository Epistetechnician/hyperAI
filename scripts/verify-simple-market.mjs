import { cpSync, mkdtempSync, rmSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const importedMarket = path.join(root, "products/simple-compute-market");
const scratch = mkdtempSync(path.join(root, ".codex-simple-market-"));
const market = path.join(scratch, "source");
const excludedNames = new Set([
  ".git",
  ".dist",
  ".mypy_cache",
  ".pytest_cache",
  ".venv",
  "build",
  "node_modules",
  "target",
  "__pycache__",
]);

const gates = [
  [
    "local core, identity, and registry wheels",
    market,
    "make",
    ["dist-core", "dist-identity", "dist-registry-client"],
  ],
  [
    "local settlement-runtime wheel",
    market,
    "make",
    ["-C", "kit/settlement-runtime", "build", "DIST_DIR=../../.dist"],
  ],
  [
    "local config wheel",
    path.join(market, "kit/config"),
    "uv",
    ["build", "--wheel", "--out-dir", "../../.dist"],
  ],
  [
    "local policy wheel",
    market,
    "make",
    ["-C", "kit/policy", "build", "DIST_DIR=../../.dist"],
  ],
  [
    "local delivery wheel",
    market,
    "make",
    ["-C", "kit/delivery", "build", "DIST_DIR=../../.dist"],
  ],
  ["core unit suites", market, "make", ["test-core"]],
  ["registry unit and integration suites", market, "make", ["test-registry"]],
  ["provisioning IaC validation tests", market, "make", ["test-provisioning-iac"]],
];

try {
  cpSync(importedMarket, market, {
    recursive: true,
    filter(source) {
      const name = path.basename(source);
      return (
        !excludedNames.has(name) &&
        !name.endsWith(".egg-info") &&
        name !== ".DS_Store" &&
        !name.endsWith(".pyc")
      );
    },
  });

  for (const [name, cwd, command, args] of gates) {
    process.stdout.write(`\n=== ${name} ===\n`);
    const result = spawnSync(command, args, {
      cwd,
      env: process.env,
      stdio: "inherit",
    });
    if (result.error) {
      process.stderr.write(`${name}: ${result.error.message}\n`);
      process.exitCode = 1;
      break;
    }
    if (result.status !== 0) {
      process.stderr.write(`${name}: exit ${result.status ?? "signal"}\n`);
      process.exitCode = result.status ?? 1;
      break;
    }
    process.stdout.write(`PASS ${name}\n`);
  }
} finally {
  rmSync(scratch, { recursive: true, force: true });
}

if (process.exitCode === undefined || process.exitCode === 0) {
  process.stdout.write("PASS bounded Simple Compute Market source suites\n");
}
