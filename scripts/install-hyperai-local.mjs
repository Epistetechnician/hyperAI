#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import * as fs from "node:fs";
import { randomUUID } from "node:crypto";
import { basename, dirname, join, parse, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function parseArguments(arguments_) {
  let requestedRoot;
  let offline = false;

  for (let index = 0; index < arguments_.length; index += 1) {
    const argument = arguments_[index];
    if (argument === "--root") {
      const candidate = arguments_[index + 1];
      if (!candidate || candidate.startsWith("--")) {
        console.error("--root requires an install directory");
        return null;
      }
      requestedRoot = candidate;
      index += 1;
    } else if (argument === "--offline") {
      offline = true;
    } else {
      console.error(`unknown installer argument: ${argument}`);
      return null;
    }
  }

  if (!requestedRoot) {
    console.error("usage: install-hyperai-local.mjs --root <install-root> [--offline]");
    return null;
  }

  return { requestedRoot, offline };
}

function existingInstallRoot(path) {
  try {
    const metadata = fs.lstatSync(path);
    if (metadata.isSymbolicLink() || !metadata.isDirectory()) {
      throw new Error("the install root must be a real directory, not a symlink or file");
    }
    return true;
  } catch (error) {
    if (error?.code === "ENOENT") return false;
    throw error;
  }
}

function installPackage(packagePath, binaries, installRoot, offline) {
  const cargoArguments = [
    "install",
    "--locked",
    "--debug",
    "--force",
    "--path",
    join(repositoryRoot, packagePath),
    "--root",
    installRoot,
  ];
  if (offline) cargoArguments.push("--offline");
  for (const binary of binaries) cargoArguments.push("--bin", binary);

  const command = process.platform === "win32" ? "cargo" : "sh";
  const commandArguments = process.platform === "win32"
    ? cargoArguments
    : ["scripts/with-pinned-rust.sh", "cargo", ...cargoArguments];
  const result = spawnSync(command, commandArguments, {
    cwd: repositoryRoot,
    stdio: "inherit",
  });
  if (result.error) {
    throw new Error(`could not start Cargo for ${packagePath}: ${result.error.message}`);
  }
  if (result.status !== 0) {
    throw new Error(`Cargo install failed for ${packagePath} with status ${result.status ?? 1}`);
  }
}

function verifyBundle(installRoot) {
  const binaryDirectory = join(installRoot, "bin");
  const extension = process.platform === "win32" ? ".exe" : "";
  for (const binary of ["hyperai", "hsai", "hyperai-ui", "hyperai-mcp"]) {
    const path = join(binaryDirectory, `${binary}${extension}`);
    try {
      const metadata = fs.lstatSync(path);
      if (metadata.isSymbolicLink() || !metadata.isFile()) {
        throw new Error("not a regular file");
      }
    } catch {
      throw new Error(`expected installed binary is missing or unsafe: ${path}`);
    }
  }
}

/** Promote a fully validated install root, restoring the prior root on failure. */
export function promoteInstallRoot(stagingRoot, installRoot, filesystem = fs) {
  let backupRoot;
  if (filesystem.existsSync(installRoot)) {
    do {
      backupRoot = join(
        dirname(installRoot),
        `.${basename(installRoot)}.backup-${randomUUID()}`,
      );
    } while (filesystem.existsSync(backupRoot));
    filesystem.renameSync(installRoot, backupRoot);
  }

  try {
    filesystem.renameSync(stagingRoot, installRoot);
  } catch (promotionError) {
    if (backupRoot) {
      try {
        filesystem.renameSync(backupRoot, installRoot);
      } catch (rollbackError) {
        throw new Error(
          `bundle promotion failed; the previous install remains at ${backupRoot}`,
          { cause: rollbackError },
        );
      }
    }
    throw promotionError;
  }

  if (backupRoot) {
    try {
      filesystem.rmSync(backupRoot, { recursive: true, force: true });
      return null;
    } catch {
      return `new bundle is installed; previous bundle cleanup remains at ${backupRoot}`;
    }
  }
  return null;
}

function install() {
  const options = parseArguments(process.argv.slice(2));
  if (!options) {
    process.exitCode = 2;
    return;
  }

  const installRoot = resolve(process.cwd(), options.requestedRoot);
  if (installRoot === parse(installRoot).root || installRoot === repositoryRoot) {
    throw new Error("choose a dedicated install root, not the filesystem or workspace root");
  }

  const hadInstall = existingInstallRoot(installRoot);
  const parentDirectory = dirname(installRoot);
  fs.mkdirSync(parentDirectory, { recursive: true });
  const stagingRoot = fs.mkdtempSync(join(parentDirectory, ".hyperai-install-stage-"));

  try {
    // Both packages are built outside the live root; neither can replace half
    // of an existing host/sidecar pair if Cargo fails.
    installPackage("crates/hyperai-mcp", ["hyperai-mcp"], stagingRoot, options.offline);
    installPackage(
      "crates/outcomes-cli",
      ["hyperai", "hsai", "hyperai-ui"],
      stagingRoot,
      options.offline,
    );
    verifyBundle(stagingRoot);

    const cleanupWarning = promoteInstallRoot(stagingRoot, installRoot);
    console.log(`Installed HyperAI and its ACP MCP sidecar under ${installRoot}`);
    if (cleanupWarning) console.error(`Warning: ${cleanupWarning}`);
    if (hadInstall) console.log("Replaced the previous local HyperAI bundle.");
  } finally {
    fs.rmSync(stagingRoot, { recursive: true, force: true });
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    install();
  } catch (error) {
    const message = error instanceof Error ? error.message : "unexpected installer failure";
    console.error(`HyperAI local install failed: ${message}`);
    process.exitCode = 1;
  }
}
