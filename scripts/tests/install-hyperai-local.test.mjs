import assert from "node:assert/strict";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { promoteInstallRoot } from "../install-hyperai-local.mjs";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const installer = join(repositoryRoot, "scripts/install-hyperai-local.mjs");
const binaryNames = ["hyperai", "hsai", "hyperai-ui", "hyperai-mcp"];

function writeBundle(root, version) {
  const bin = join(root, "bin");
  mkdirSync(bin, { recursive: true });
  for (const name of binaryNames) {
    const path = join(bin, name);
    writeFileSync(path, `${version}:${name}\n`);
    chmodSync(path, 0o755);
  }
}

function bundleBytes(root) {
  return Object.fromEntries(
    binaryNames.map((name) => [name, readFileSync(join(root, "bin", name))]),
  );
}

function createFakeCargoHome(root) {
  const cargoHome = join(root, "fake-cargo-home");
  const toolchainDirectory = join(root, "fake-toolchain");
  const rustupBin = join(cargoHome, "bin", "rustup");
  const cargoBin = join(toolchainDirectory, "cargo");
  mkdirSync(dirname(rustupBin), { recursive: true });
  mkdirSync(toolchainDirectory, { recursive: true });

  writeFileSync(
    rustupBin,
    `#!/usr/bin/env node\nif (process.argv[2] !== "which") process.exit(91);\nprocess.stdout.write(${JSON.stringify(cargoBin)} + "\\n");\n`,
  );
  chmodSync(rustupBin, 0o755);

  writeFileSync(
    cargoBin,
    `#!/usr/bin/env node
const { chmodSync, mkdirSync, writeFileSync } = require("node:fs");
const { join } = require("node:path");
const args = process.argv.slice(2);
const valueAfter = (flag) => args[args.indexOf(flag) + 1];
const packagePath = valueAfter("--path");
if (process.env.HYPERAI_INSTALL_TEST_FAIL_PACKAGE === packagePath.split("/").at(-1)) {
  process.exit(37);
}
const root = valueAfter("--root");
const binaries = args.flatMap((arg, index) => arg === "--bin" ? [args[index + 1]] : []);
const directory = join(root, "bin");
mkdirSync(directory, { recursive: true });
for (const binary of binaries) {
  const path = join(directory, binary);
  writeFileSync(path, process.env.HYPERAI_INSTALL_TEST_VERSION + ":" + binary + "\\n");
  chmodSync(path, 0o755);
}
`,
  );
  chmodSync(cargoBin, 0o755);
  return cargoHome;
}

function runInstaller(installRoot, cargoHome, version, failPackage) {
  return spawnSync(process.execPath, [installer, "--root", installRoot, "--offline"], {
    cwd: repositoryRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      CARGO_HOME: cargoHome,
      HYPERAI_INSTALL_TEST_VERSION: version,
      HYPERAI_INSTALL_TEST_FAIL_PACKAGE: failPackage ?? "",
    },
  });
}

test("installer stages the whole bundle and preserves it when the second package fails", (context) => {
  const temporary = mkdtempSync(join(tmpdir(), "hyperai-install-transaction-"));
  context.after(() => rmSync(temporary, { recursive: true, force: true }));
  const installRoot = join(temporary, "install-root");
  const dataHome = join(temporary, "home");
  const cargoHome = createFakeCargoHome(temporary);
  mkdirSync(dataHome, { recursive: true });
  const sentinel = join(dataHome, "user-data-sentinel");
  writeFileSync(sentinel, "preserve local HyperAI data\n");

  const first = runInstaller(installRoot, cargoHome, "v1");
  assert.equal(first.status, 0, first.stderr);
  assert.deepEqual(
    Object.fromEntries(Object.entries(bundleBytes(installRoot)).map(([name, bytes]) => [name, bytes.toString()])),
    Object.fromEntries(binaryNames.map((name) => [name, `v1:${name}\n`])),
  );
  const oldBundle = bundleBytes(installRoot);

  const failedUpgrade = runInstaller(installRoot, cargoHome, "v2", "outcomes-cli");
  assert.equal(failedUpgrade.status, 1, failedUpgrade.stderr);
  assert.match(failedUpgrade.stderr, /Cargo install failed for crates\/outcomes-cli/);
  assert.deepEqual(bundleBytes(installRoot), oldBundle);
  assert.equal(readFileSync(sentinel, "utf8"), "preserve local HyperAI data\n");

  const successfulUpgrade = runInstaller(installRoot, cargoHome, "v2");
  assert.equal(successfulUpgrade.status, 0, successfulUpgrade.stderr);
  assert.deepEqual(
    Object.fromEntries(Object.entries(bundleBytes(installRoot)).map(([name, bytes]) => [name, bytes.toString()])),
    Object.fromEntries(binaryNames.map((name) => [name, `v2:${name}\n`])),
  );
  assert.equal(readFileSync(sentinel, "utf8"), "preserve local HyperAI data\n");
  assert.deepEqual(
    readdirSync(temporary).filter((name) => name.startsWith(".hyperai-install-stage-")),
    [],
  );
  assert.deepEqual(
    readdirSync(temporary).filter((name) => name.startsWith(".install-root.backup-")),
    [],
  );
});

test("promotion failure restores the previous install root", (context) => {
  const temporary = mkdtempSync(join(tmpdir(), "hyperai-promote-rollback-"));
  context.after(() => rmSync(temporary, { recursive: true, force: true }));
  const installRoot = join(temporary, "install-root");
  const stagingRoot = join(temporary, "staging-root");
  writeBundle(installRoot, "v1");
  writeBundle(stagingRoot, "v2");
  const oldBundle = bundleBytes(installRoot);

  const filesystem = {
    existsSync,
    rmSync,
    renameSync(from, to) {
      if (from === stagingRoot && to === installRoot) {
        throw new Error("injected promotion failure");
      }
      return renameSync(from, to);
    },
  };

  assert.throws(
    () => promoteInstallRoot(stagingRoot, installRoot, filesystem),
    /injected promotion failure/,
  );
  assert.deepEqual(bundleBytes(installRoot), oldBundle);
  assert.deepEqual(
    readdirSync(temporary).filter((name) => name.startsWith(".install-root.backup-")),
    [],
  );
});

test("failed rollback preserves and reports the previous bundle backup", (context) => {
  const temporary = mkdtempSync(join(tmpdir(), "hyperai-promote-recovery-"));
  context.after(() => rmSync(temporary, { recursive: true, force: true }));
  const installRoot = join(temporary, "install-root");
  const stagingRoot = join(temporary, "staging-root");
  writeBundle(installRoot, "v1");
  writeBundle(stagingRoot, "v2");
  let previousRootMoved = false;

  const filesystem = {
    existsSync,
    rmSync,
    renameSync(from, to) {
      if (from === installRoot && to.includes(".install-root.backup-")) {
        previousRootMoved = true;
      } else if (from === stagingRoot && to === installRoot) {
        throw new Error("injected promotion failure");
      } else if (previousRootMoved && to === installRoot) {
        throw new Error("injected rollback failure");
      }
      return renameSync(from, to);
    },
  };

  assert.throws(
    () => promoteInstallRoot(stagingRoot, installRoot, filesystem),
    (error) => {
      assert.match(error.message, /previous install remains at/);
      const backupRoot = error.message.split("previous install remains at ")[1];
      assert.deepEqual(
        Object.fromEntries(
          Object.entries(bundleBytes(backupRoot)).map(([name, bytes]) => [name, bytes.toString()]),
        ),
        Object.fromEntries(binaryNames.map((name) => [name, `v1:${name}\n`])),
      );
      assert.equal(readFileSync(join(backupRoot, "bin", "hyperai"), "utf8"), "v1:hyperai\n");
      return true;
    },
  );
  assert.equal(existsSync(installRoot), false);
});
