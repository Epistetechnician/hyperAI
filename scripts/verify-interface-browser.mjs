#!/usr/bin/env node

import { execFileSync, spawn } from "node:child_process";
import * as fs from "node:fs";
import { randomUUID } from "node:crypto";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import {
  hasLiveProcessGroupMembers,
  readProcessGroupMembers,
} from "./browser-process-group.mjs";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const serviceBinary = path.join(repositoryRoot, "target/debug/hyperai-ui");
const routes = [
  ["overview", "Overview"],
  ["market", "Market"],
  ["deals", "Deals"],
  ["data", "Data"],
  ["inference", "Inference"],
  ["fleet", "Fleet"],
  ["eval", "Eval"],
  ["trade", "Trade"],
  ["evidence", "Evidence"],
  ["access", "Agents & access"],
  ["activity", "Activity"],
];
const viewports = [
  { name: "desktop", width: 1440, height: 1000, mobile: false },
  { name: "zoom-200-equivalent", width: 720, height: 500, mobile: false },
  { name: "mobile", width: 390, height: 844, mobile: true },
];
const screenshotTargets = new Set([
  "desktop:overview",
  "zoom-200-equivalent:data",
  "mobile:deals",
]);

function parseArguments(arguments_) {
  arguments_ = arguments_.filter((argument) => argument !== "--");
  const retainScreenshots = arguments_.includes("--retain-screenshots");
  const auditContrast = arguments_.includes("--audit-contrast");
  const reconciliationStdin = arguments_.includes("--reconciliation-stdin");
  const dataWorkflow = arguments_.includes("--data-workflow");
  const gpudiffScheduleWorkflow = arguments_.includes("--gpudiff-schedule-workflow");
  const dataFailureWorkflow = arguments_.includes("--data-failure-workflow");
  const dataKeyboardViewWorkflow = arguments_.includes("--data-keyboard-view-workflow");
  const dataFullKeyboardWorkflow = arguments_.includes("--data-full-keyboard-workflow");
  const dataFullKeyboardAXWorkflow = arguments_.includes("--data-full-keyboard-ax-workflow");
  const dataFullKeyboardAXSemanticWorkflow = arguments_.includes("--data-full-keyboard-ax-semantic-workflow");
  const marketSupplyComparisonWorkflow = arguments_.includes("--market-supply-comparison-workflow");
  const marketSupplyQuoteWorkflow = arguments_.includes("--market-supply-quote-workflow");
  const marketSupplyQuoteLifecycleWorkflow = arguments_.includes("--market-supply-quote-lifecycle-workflow");
  const marketStaleSupplyQuoteWorkflow = arguments_.includes("--market-stale-supply-quote-workflow");
  const workloadTraceSemanticWorkflow = arguments_.includes("--workload-trace-semantic-workflow");
  const sessionActivityOwnerLinksWorkflow = arguments_.includes("--session-activity-owner-links-workflow");
  const acpKeyboardAXWorkflow = arguments_.includes("--acp-keyboard-ax-workflow");
  const acpCancelKeyboardWorkflow = arguments_.includes("--acp-cancel-keyboard-workflow");
  const taskNavigationKeyboardWorkflow = arguments_.includes("--task-navigation-keyboard-workflow");
  if (arguments_.some((argument) => ![
    "--retain-screenshots",
    "--audit-contrast",
    "--reconciliation-stdin",
    "--data-workflow",
    "--gpudiff-schedule-workflow",
    "--data-failure-workflow",
    "--data-keyboard-view-workflow",
    "--data-full-keyboard-workflow",
    "--data-full-keyboard-ax-workflow",
    "--data-full-keyboard-ax-semantic-workflow",
    "--market-supply-comparison-workflow",
    "--market-supply-quote-workflow",
    "--market-supply-quote-lifecycle-workflow",
    "--market-stale-supply-quote-workflow",
    "--workload-trace-semantic-workflow",
    "--session-activity-owner-links-workflow",
    "--acp-keyboard-ax-workflow",
    "--acp-cancel-keyboard-workflow",
    "--task-navigation-keyboard-workflow",
  ].includes(argument))) {
    throw new Error("usage: verify-interface-browser.mjs [--retain-screenshots] [--audit-contrast] [--reconciliation-stdin | --data-workflow | --gpudiff-schedule-workflow | --data-failure-workflow | --data-keyboard-view-workflow | --data-full-keyboard-workflow | --data-full-keyboard-ax-workflow | --data-full-keyboard-ax-semantic-workflow | --market-supply-comparison-workflow | --market-supply-quote-workflow | --market-supply-quote-lifecycle-workflow | --market-stale-supply-quote-workflow | --workload-trace-semantic-workflow | --session-activity-owner-links-workflow | --acp-keyboard-ax-workflow | --acp-cancel-keyboard-workflow | --task-navigation-keyboard-workflow]");
  }
  if (Number(reconciliationStdin) + Number(dataWorkflow) + Number(gpudiffScheduleWorkflow) + Number(dataFailureWorkflow)
    + Number(dataKeyboardViewWorkflow) + Number(dataFullKeyboardWorkflow)
    + Number(dataFullKeyboardAXWorkflow) + Number(dataFullKeyboardAXSemanticWorkflow)
    + Number(marketSupplyComparisonWorkflow)
    + Number(marketSupplyQuoteWorkflow) + Number(marketSupplyQuoteLifecycleWorkflow)
    + Number(marketStaleSupplyQuoteWorkflow)
    + Number(workloadTraceSemanticWorkflow) + Number(sessionActivityOwnerLinksWorkflow)
    + Number(acpKeyboardAXWorkflow)
    + Number(acpCancelKeyboardWorkflow) + Number(taskNavigationKeyboardWorkflow) > 1) {
    throw new Error("reconciliation, Data, GPUDiff schedule, Market supply comparison/quote, Workload Trace, session Activity, and ACP keyboard workflows are separate browser runs");
  }
  return {
    retainScreenshots,
    auditContrast,
    reconciliationStdin,
    dataWorkflow,
    gpudiffScheduleWorkflow,
    dataFailureWorkflow,
    dataKeyboardViewWorkflow,
    dataFullKeyboardWorkflow,
    dataFullKeyboardAXWorkflow,
    dataFullKeyboardAXSemanticWorkflow,
    marketSupplyComparisonWorkflow,
    marketSupplyQuoteWorkflow,
    marketSupplyQuoteLifecycleWorkflow,
    marketStaleSupplyQuoteWorkflow,
    workloadTraceSemanticWorkflow,
    sessionActivityOwnerLinksWorkflow,
    acpKeyboardAXWorkflow,
    acpCancelKeyboardWorkflow,
    taskNavigationKeyboardWorkflow,
  };
}

function contrastAuditExpression() {
  return `(() => {
    const parseColor = (value) => {
      const parts = String(value).match(/[\\d.]+/g)?.map(Number) || [];
      if (parts.length < 3 || parts.slice(0, 3).some((part) => part < 0 || part > 255)) return null;
      return [parts[0], parts[1], parts[2], parts.length > 3 ? parts[3] : 1];
    };
    const composite = (foreground, background) => {
      const alpha = foreground[3];
      return [
        foreground[0] * alpha + background[0] * (1 - alpha),
        foreground[1] * alpha + background[1] * (1 - alpha),
        foreground[2] * alpha + background[2] * (1 - alpha),
        alpha + background[3] * (1 - alpha),
      ];
    };
    const luminance = (color) => {
      const channels = color.slice(0, 3).map((channel) => {
        const value = channel / 255;
        return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
      });
      return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
    };
    const ratio = (foreground, background) => {
      const values = [luminance(foreground), luminance(background)].sort((a, b) => b - a);
      return (values[0] + 0.05) / (values[1] + 0.05);
    };
    const describe = (element) => {
      const path = [];
      for (let node = element; node && node !== document.body && path.length < 4; node = node.parentElement) {
        const classes = typeof node.className === "string"
          ? node.className.trim().split(/\\s+/).filter(Boolean).slice(0, 3)
          : [];
        if (node.id || classes.length > 0 || node === element) {
          path.unshift(node.tagName.toLowerCase() + (node.id ? "#" + node.id : "")
            + classes.map((name) => "." + name).join(""));
        }
      }
      return path.join(" > ");
    };
    const failures = [];
    const reviewSamples = [];
    let checked = 0;
    let reviewed = 0;
    let excluded = 0;
    let minimum = null;
    const elements = Array.from(document.querySelectorAll("body *"));
    for (const element of elements) {
      const directText = Array.from(element.childNodes)
        .filter((node) => node.nodeType === Node.TEXT_NODE)
        .map((node) => node.textContent || "")
        .join(" ")
        .replace(/\\s+/g, " ")
        .trim();
      if (!directText || ["SCRIPT", "STYLE", "NOSCRIPT"].includes(element.tagName)) continue;
      if (element.closest('[hidden], [aria-hidden="true"], [inert]') || element.closest(":disabled")) {
        excluded += 1;
        continue;
      }
      if (element.closest(".brand")) {
        excluded += 1;
        continue;
      }
      const rect = element.getBoundingClientRect();
      const ownStyle = getComputedStyle(element);
      if (!element.getClientRects().length || rect.width === 0 || rect.height === 0
        || ownStyle.display === "none" || ownStyle.visibility === "hidden") {
        excluded += 1;
        continue;
      }

      let background = [255, 255, 255, 1];
      let reviewReason = "";
      const ancestors = [];
      for (let node = element; node; node = node.parentElement) ancestors.push(node);
      for (const node of ancestors.reverse()) {
        const style = getComputedStyle(node);
        if (style.backgroundImage !== "none") {
          reviewReason = "background image or gradient";
          break;
        }
        if (Math.abs(Number.parseFloat(style.opacity) - 1) > 0.0001) {
          reviewReason = "inherited opacity";
          break;
        }
        if (style.mixBlendMode !== "normal") {
          reviewReason = "non-normal blend mode";
          break;
        }
        const layer = parseColor(style.backgroundColor);
        if (!layer) {
          reviewReason = "unparsed computed background";
          break;
        }
        background = composite(layer, background);
      }
      const foreground = parseColor(ownStyle.color);
      if (!reviewReason && !foreground) reviewReason = "unparsed computed text color";
      if (reviewReason) {
        reviewed += 1;
        if (reviewSamples.length < 12) {
          reviewSamples.push({ selector: describe(element), text: directText.slice(0, 80), reason: reviewReason });
        }
        continue;
      }

      const size = Number.parseFloat(ownStyle.fontSize);
      const weight = Number.parseInt(ownStyle.fontWeight, 10) || 400;
      const largeText = size >= 24 || (size >= 18.6667 && weight >= 700);
      const threshold = largeText ? 3 : 4.5;
      const effectiveForeground = composite(foreground, background).slice(0, 3);
      const effectiveBackground = background.slice(0, 3);
      const value = ratio(effectiveForeground, effectiveBackground);
      const sample = {
        selector: describe(element),
        text: directText.slice(0, 80),
        ratio: Number(value.toFixed(4)),
        threshold,
        foreground: ownStyle.color,
        background: "rgb(" + effectiveBackground.map((channel) => Math.round(channel)).join(", ") + ")",
      };
      checked += 1;
      if (!minimum || value < minimum.ratio) minimum = { ...sample, ratio: value };
      if (value < threshold) failures.push(sample);
    }
    failures.sort((a, b) => a.ratio - b.ratio);
    if (minimum) minimum.ratio = Number(minimum.ratio.toFixed(4));
    return { checked, reviewed, excluded, minimum, failureCount: failures.length, failures, reviewSamples };
  })()`;
}

function findChrome() {
  const candidates = [
    process.env.CHROME_BIN,
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "/opt/homebrew/bin/chromium",
    "/usr/bin/chromium",
    "/usr/bin/google-chrome",
  ].filter(Boolean);
  const binary = candidates.find((candidate) => fs.existsSync(candidate));
  if (!binary) {
    throw new Error("Google Chrome or Chromium was not found; set CHROME_BIN to its executable");
  }
  return binary;
}

function reserveLoopbackPort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        server.close();
        reject(new Error("could not reserve an IPv4 loopback port"));
        return;
      }
      server.close((error) => error ? reject(error) : resolve(address.port));
    });
  });
}

function loopbackPortIsClosed(port) {
  return new Promise((resolve) => {
    const socket = net.createConnection({ host: "127.0.0.1", port });
    let settled = false;
    const finish = (closed) => {
      if (settled) return;
      settled = true;
      socket.destroy();
      resolve(closed);
    };
    socket.once("connect", () => finish(false));
    socket.once("error", (error) => finish(error?.code === "ECONNREFUSED"));
    socket.setTimeout(750, () => finish(false));
  });
}

function childHasExited(child) {
  return !child || child.exitCode !== null || child.signalCode !== null;
}

function childExitDescription(child) {
  return child?.signalCode
    ? "signal " + child.signalCode
    : "status " + String(child?.exitCode ?? "unknown");
}

function waitForExit(child, timeoutMs) {
  if (childHasExited(child)) return Promise.resolve(true);
  return new Promise((resolve) => {
    const timer = setTimeout(() => resolve(false), timeoutMs);
    child.once("exit", () => {
      clearTimeout(timer);
      resolve(true);
    });
  });
}

function processGroupHasLiveMembers(processGroupId) {
  return hasLiveProcessGroupMembers(readProcessGroupMembers(processGroupId));
}

async function waitForProcessGroupEmpty(processGroupId, timeoutMs, stableMs = 750) {
  const deadline = Date.now() + timeoutMs;
  let absentSince;
  while (Date.now() < deadline) {
    if (processGroupHasLiveMembers(processGroupId)) {
      absentSince = undefined;
    } else if (absentSince === undefined) {
      absentSince = Date.now();
    } else if (Date.now() - absentSince >= stableMs) {
      return true;
    }
    await delay(50);
  }
  return false;
}

async function stopProcessGroup(processGroupId) {
  const signalGroup = (signal) => {
    try {
      process.kill(-processGroupId, signal);
    } catch (error) {
      if (error?.code !== "ESRCH") throw error;
    }
  };

  signalGroup("SIGTERM");
  if (await waitForProcessGroupEmpty(processGroupId, 5000)) return;
  signalGroup("SIGKILL");
  if (!await waitForProcessGroupEmpty(processGroupId, 3000)) {
    throw new Error(`runner-owned browser process group ${processGroupId} did not exit before cleanup`);
  }
}

async function stopProcess(child, processGroup = false) {
  if (!child?.pid || (!processGroup && childHasExited(child))) return;
  const scopeExists = () => processGroup
    ? processGroupHasLiveMembers(child.pid)
    : !childHasExited(child);
  if (!scopeExists()) return;
  try {
    process.kill(processGroup ? -child.pid : child.pid, "SIGTERM");
  } catch (error) {
    if (error?.code !== "ESRCH") throw error;
  }
  const waitForScopeExit = (timeoutMs) => processGroup
    ? waitForProcessGroupEmpty(child.pid, timeoutMs)
    : waitForExit(child, timeoutMs);
  if (await waitForScopeExit(5000)) return;
  try {
    process.kill(processGroup ? -child.pid : child.pid, "SIGKILL");
  } catch (error) {
    if (error?.code !== "ESRCH") throw error;
  }
  if (!await waitForScopeExit(3000)) {
    throw new Error(processGroup
      ? "the runner-owned browser process group did not exit before cleanup"
      : "the runner-owned child process did not exit before cleanup");
  }
}

async function waitForHttp(url, child, timeoutMs = 15000) {
  const deadline = Date.now() + timeoutMs;
  let lastError = "no response";
  while (Date.now() < deadline) {
    if (childHasExited(child)) {
      throw new Error("the disposable interface service exited with " + childExitDescription(child));
    }
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(1000) });
      if (response.status === 200) return response;
      lastError = `HTTP ${response.status}`;
    } catch (error) {
      lastError = error instanceof Error ? error.message : "request failed";
    }
    await delay(100);
  }
  throw new Error(`the disposable interface service did not become ready: ${lastError}`);
}

async function waitForDevTools(port, child, timeoutMs = 15000) {
  const endpoint = `http://127.0.0.1:${port}/json/list`;
  const deadline = Date.now() + timeoutMs;
  let lastError = "no response";
  while (Date.now() < deadline) {
    if (childHasExited(child)) {
      throw new Error("headless Chrome exited with " + childExitDescription(child));
    }
    try {
      const response = await fetch(endpoint, { signal: AbortSignal.timeout(1000) });
      if (response.ok) {
        const targets = await response.json();
        const page = targets.find((target) => target.type === "page" && target.webSocketDebuggerUrl);
        if (page) return page;
      }
      lastError = `HTTP ${response.status}`;
    } catch (error) {
      lastError = error instanceof Error ? error.message : "request failed";
    }
    await delay(100);
  }
  throw new Error(`headless Chrome DevTools did not become ready: ${lastError}`);
}

async function waitForBrowserWebSocketUrl(port, child, timeoutMs = 15000) {
  const endpoint = `http://127.0.0.1:${port}/json/version`;
  const deadline = Date.now() + timeoutMs;
  let lastError = "no response";
  while (Date.now() < deadline) {
    if (childHasExited(child)) {
      throw new Error("headless Chrome exited with " + childExitDescription(child));
    }
    try {
      const response = await fetch(endpoint, { signal: AbortSignal.timeout(1000) });
      if (response.ok) {
        const version = await response.json();
        if (typeof version.webSocketDebuggerUrl === "string") return version.webSocketDebuggerUrl;
      }
      lastError = `HTTP ${response.status}`;
    } catch (error) {
      lastError = error instanceof Error ? error.message : "request failed";
    }
    await delay(100);
  }
  throw new Error(`headless Chrome browser DevTools did not become ready: ${lastError}`);
}

function assertLoopbackListener(port) {
  const output = execFileSync("lsof", ["-nP", `-iTCP:${port}`, "-sTCP:LISTEN"], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
  });
  const listeners = output.split(/\r?\n/).filter((line) => line.includes("LISTEN"));
  if (listeners.length !== 1 || !listeners[0].includes(`127.0.0.1:${port} (LISTEN)`)) {
    throw new Error(`port ${port} is not owned by exactly one IPv4 loopback listener`);
  }
  return listeners[0].trim().split(/\s+/).slice(0, 2).join(" ");
}

class DevToolsPage {
  constructor(webSocketUrl) {
    this.socket = new WebSocket(webSocketUrl.replace("localhost", "127.0.0.1"));
    this.nextId = 0;
    this.pending = new Map();
    this.listeners = new Map();
    this.opened = new Promise((resolve, reject) => {
      this.socket.addEventListener("open", resolve, { once: true });
      this.socket.addEventListener("error", () => reject(new Error("DevTools WebSocket failed to open")), { once: true });
    });
    this.socket.addEventListener("message", (event) => this.#onMessage(event.data));
    this.socket.addEventListener("close", () => {
      for (const entry of this.pending.values()) entry.reject(new Error("DevTools WebSocket closed"));
      this.pending.clear();
    });
  }

  #onMessage(data) {
    let message;
    try {
      message = JSON.parse(String(data));
    } catch {
      return;
    }
    if (message.id !== undefined) {
      const pending = this.pending.get(message.id);
      if (!pending) return;
      this.pending.delete(message.id);
      clearTimeout(pending.timer);
      if (message.error) pending.reject(new Error(`DevTools command failed: ${message.error.message}`));
      else pending.resolve(message.result || {});
      return;
    }
    for (const listener of this.listeners.get(message.method) || []) listener(message.params || {});
  }

  on(method, listener) {
    const entries = this.listeners.get(method) || [];
    entries.push(listener);
    this.listeners.set(method, entries);
  }

  async send(method, params = {}) {
    await this.opened;
    const id = ++this.nextId;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`DevTools command timed out: ${method}`));
      }, 10000);
      this.pending.set(id, { resolve, reject, timer });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }

  async evaluate(expression) {
    const result = await this.send("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
      userGesture: true,
    });
    if (result.exceptionDetails) {
      const details = result.exceptionDetails.exception?.description || result.exceptionDetails.text || "runtime exception";
      const callSite = new Error().stack?.split("\n").slice(2)
        .map((frame) => frame.match(/verify-interface-browser\.mjs:(\d+):(\d+)/))
        .find(Boolean);
      const pageLine = Number.isInteger(result.exceptionDetails.lineNumber)
        ? result.exceptionDetails.lineNumber
        : "unknown";
      const pageColumn = Number.isInteger(result.exceptionDetails.columnNumber)
        ? result.exceptionDetails.columnNumber
        : "unknown";
      throw new Error(`browser evaluation failed at harness line ${callSite?.[1] ?? "unknown"} `
        + `(page line ${pageLine}, column ${pageColumn}): ${String(details).slice(0, 600)}`);
    }
    return result.result?.value;
  }

  close() {
    if (this.socket.readyState === WebSocket.OPEN || this.socket.readyState === WebSocket.CONNECTING) {
      this.socket.close();
    }
  }
}

async function waitForPageHeading(page) {
  const deadline = Date.now() + 10000;
  while (Date.now() < deadline) {
    const ready = await page.evaluate(`({
      complete: document.readyState === "complete",
      heading: Boolean(document.querySelector("#page-root h1")),
    })`);
    if (ready?.complete && ready.heading) return;
    await delay(50);
  }
  throw new Error("the interface did not render its primary heading");
}

async function readReconciliationStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) chunks.push(Buffer.from(chunk));
  let input;
  try {
    input = JSON.parse(Buffer.concat(chunks).toString("utf8"));
  } catch {
    throw new Error("reconciliation stdin must be one JSON request object");
  }
  if (typeof input?.execution_input !== "string" || typeof input?.trusted_keys_input !== "string") {
    throw new Error("reconciliation stdin is missing the versioned input documents");
  }
  if (Buffer.byteLength(input.execution_input) + Buffer.byteLength(input.trusted_keys_input) > 150_000) {
    throw new Error("reconciliation stdin exceeds the local adapter input limit");
  }
  return input;
}

async function exerciseReconciliationFlow(page, input) {
  const inputLiteral = JSON.stringify(input);
  const streamCanary = "HYPERAI_SYNTHETIC_RECONCILIATION_STREAM_CANARY";
  await page.send("Emulation.setDeviceMetricsOverride", {
    width: 1440,
    height: 1000,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await page.evaluate('document.querySelector(".primary-nav [data-page=inference]")?.click()');
  await delay(80);
  const submitted = await page.evaluate(`(() => {
    const input = ${inputLiteral};
    const attach = (id, name, contents) => {
      const control = document.getElementById(id);
      if (!control) return false;
      const transfer = new DataTransfer();
      transfer.items.add(new File([contents], name, { type: "application/json" }));
      control.files = transfer.files;
      return control.files.length === 1;
    };
    const executionAttached = attach(
      "hyperintel-execution-file",
      "synthetic-execution.json",
      input.execution_input,
    );
    const trustAttached = attach(
      "hyperintel-reconciliation-trust-file",
      "synthetic-trust.json",
      input.trusted_keys_input,
    );
    const button = document.querySelector('[data-action="reconcile-hyperintel"]');
    if (!executionAttached || !trustAttached || !button) return { submitted: false };
    button.click();
    return { submitted: true };
  })()`);
  if (!submitted?.submitted) {
    throw new Error("the Inference form did not accept both in-memory synthetic files");
  }

  let inferenceResult;
  const deadline = Date.now() + 15000;
  while (Date.now() < deadline && !inferenceResult) {
    inferenceResult = await page.evaluate(`(() => {
      const panel = Array.from(document.querySelectorAll(".hyperintel-result"))
        .find((element) => element.querySelector("h2")?.textContent.trim() === "Execution lineage reconstructed");
      if (!panel) return null;
      const detail = Array.from(panel.querySelectorAll(".detail-list dt"));
      const valueFor = (label) => {
        const item = detail.find((element) => element.textContent.trim() === label);
        return item?.nextElementSibling?.textContent.trim() || "";
      };
      const digest = panel.querySelector('.detail-list dt')
        ? valueFor("Report digest")
        : "";
      const authority = valueFor("Authority");
      const reportStatus = panel.querySelector(".verification-metrics strong")?.textContent.trim() || "";
      const storageValues = [];
      for (const storage of [localStorage, sessionStorage]) {
        for (let index = 0; index < storage.length; index += 1) {
          const key = storage.key(index);
          storageValues.push(key || "", storage.getItem(key) || "");
        }
      }
      const serializedDom = document.documentElement.outerHTML;
      const visibleText = document.documentElement.textContent || "";
      return {
        title: panel.querySelector("h2")?.textContent.trim() || "",
        reportStatus,
        digest: digest.replace(/\\s+/g, ""),
        authority,
        claimCeiling: panel.querySelector(".claim-box")?.textContent.trim() || "",
        canaryInDom: serializedDom.includes(${JSON.stringify(streamCanary)})
          || visibleText.includes(${JSON.stringify(streamCanary)}),
        canaryInStorage: storageValues.some((value) => value.includes(${JSON.stringify(streamCanary)})),
      };
    })()`);
    if (!inferenceResult) await delay(100);
  }
  if (!inferenceResult) throw new Error("the populated Inference report did not render");
  if (!/^[0-9a-f]{64}$/.test(inferenceResult.digest)
    || inferenceResult.reportStatus !== "COMPLETED"
    || inferenceResult.authority !== "None"
    || inferenceResult.canaryInDom
    || inferenceResult.canaryInStorage
    || !inferenceResult.claimCeiling) {
    throw new Error("the populated Inference report violated its content-free claim contract");
  }

  await page.evaluate('document.querySelector(".primary-nav [data-page=activity]")?.click()');
  await delay(80);
  const activityResult = await page.evaluate(`(() => {
    const card = Array.from(document.querySelectorAll(".session-record"))
      .find((element) => element.querySelector("h3")?.textContent.trim() === "Execution-lineage report");
    const digest = card?.querySelector("code")?.textContent.trim() || "";
    return {
      source: card?.querySelector(".eyebrow")?.textContent.trim() || "",
      title: card?.querySelector("h3")?.textContent.trim() || "",
      digest,
      status: card?.querySelector(".tag")?.textContent.trim() || "",
      note: card?.querySelector("small")?.textContent.trim() || "",
      returnLink: card?.querySelector('button[data-page="inference"]')?.textContent.trim() || "",
    };
  })()`);
  if (activityResult.source !== "HYPERINTEL"
    || activityResult.title !== "Execution-lineage report"
    || activityResult.digest !== inferenceResult.digest
    || activityResult.status !== "LOCAL RECONCILIATION"
    || !activityResult.note.includes("does not establish correct computation")
    || activityResult.returnLink !== "Open Inference") {
    throw new Error("Activity did not link the source report and digest back to Inference");
  }

  await page.send("Emulation.setDeviceMetricsOverride", {
    width: 390,
    height: 844,
    deviceScaleFactor: 1,
    mobile: true,
  });
  await delay(120);
  const mobileActivity = await page.evaluate(`(() => ({
    clientWidth: document.documentElement.clientWidth,
    scrollWidth: document.documentElement.scrollWidth,
    reportVisible: Array.from(document.querySelectorAll(".session-record h3"))
      .some((element) => element.textContent.trim() === "Execution-lineage report"),
  }))()`);
  if (!mobileActivity.reportVisible || mobileActivity.scrollWidth > mobileActivity.clientWidth) {
    throw new Error("the populated Activity card overflows or disappears at the mobile viewport");
  }
  await page.evaluate(`Array.from(document.querySelectorAll(".session-record"))
    .find((element) => element.querySelector("h3")?.textContent.trim() === "Execution-lineage report")
    ?.querySelector('button[data-page="inference"]')?.click()`);
  await delay(80);
  const returnedToInference = await page.evaluate(`(() => ({
    heading: document.querySelector("#workspace-page-title")?.textContent.trim() || "",
    reportVisible: Array.from(document.querySelectorAll(".hyperintel-result h2"))
      .some((element) => element.textContent.trim() === "Execution lineage reconstructed"),
    clientWidth: document.documentElement.clientWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }))()`);
  if (returnedToInference.heading !== "Inference"
    || !returnedToInference.reportVisible
    || returnedToInference.scrollWidth > returnedToInference.clientWidth) {
    throw new Error("the Activity return link did not restore the populated Inference view");
  }
  return {
    status: "passed",
    viewport: { width: mobileActivity.clientWidth, height: 844 },
    reportStatus: inferenceResult.reportStatus,
    reportDigest: inferenceResult.digest,
    authority: inferenceResult.authority,
    activitySource: activityResult.source,
    activityStatus: activityResult.status,
    activityDigestMatches: activityResult.digest === inferenceResult.digest,
    returnedToInference: returnedToInference.heading === "Inference",
    canaryAbsentFromDomAndStorage: !inferenceResult.canaryInDom && !inferenceResult.canaryInStorage,
  };
}

async function waitForBrowserValue(page, expression, description, predicate, timeoutMs = 15000) {
  const deadline = Date.now() + timeoutMs;
  let lastValue;
  while (Date.now() < deadline) {
    lastValue = await page.evaluate(expression);
    if (predicate(lastValue)) return lastValue;
    await delay(50);
  }
  throw new Error(`${description} timed out (${JSON.stringify(lastValue)})`);
}

async function submitDataForm(page, formId, values) {
  return page.evaluate(`(() => {
    const form = document.getElementById(${JSON.stringify(formId)});
    const values = ${JSON.stringify(values)};
    if (!form) return false;
    for (const [name, value] of Object.entries(values)) {
      const field = form.elements.namedItem(name);
      if (!field) return false;
      field.value = value;
    }
    form.requestSubmit();
    return true;
  })()`);
}

async function submitDataQuery(page, sql, statusPrefix) {
  const submitted = await page.evaluate(`(() => {
    const form = document.getElementById("data-query-form");
    const field = document.getElementById("data-sql");
    if (!form || !field) return false;
    field.value = ${JSON.stringify(sql)};
    form.requestSubmit();
    return true;
  })()`);
  if (!submitted) throw new Error("the populated Data query form was not available");
  return waitForBrowserValue(page, `(() => {
    const status = document.getElementById("data-query-status");
    if (!status) return null;
    return {
      text: status.textContent.trim(),
      role: status.getAttribute("role"),
      live: status.getAttribute("aria-live"),
      focusTarget: document.activeElement?.id || "",
    };
  })()`, `Data query status ${statusPrefix}`, (value) => value?.text.startsWith(statusPrefix));
}

async function readDataWorkflowState(page) {
  return page.evaluate(`(() => {
    const result = document.querySelector(".data-result");
    const table = result?.querySelector(".data-result-table");
    const rows = table ? Array.from(table.querySelectorAll("tbody tr")) : [];
    const resultHeading = result?.querySelector("h2")?.textContent.trim() || "";
    const metaCodes = result ? Array.from(result.querySelectorAll(".data-result-meta code")) : [];
    const provenance = Object.fromEntries(Array.from(document.querySelectorAll(".data-provenance dt"))
      .map((term) => [term.textContent.trim(), term.nextElementSibling?.textContent.trim() || ""]));
    const queryStatus = document.getElementById("data-query-status");
    return {
      datasetId: result?.querySelector(".data-result-meta code")?.textContent.trim() || "",
      snapshotDigest: metaCodes[1]?.title || "",
      rowCount: rows.length,
      resultHeading,
      source: provenance.Source || "",
      queryStatus: queryStatus?.textContent.trim() || "",
      queryStatusRole: queryStatus?.getAttribute("role") || "",
      queryStatusLive: queryStatus?.getAttribute("aria-live") || "",
      queryStatusFocus: document.activeElement?.id === "data-query-status",
    };
  })()`);
}

async function assertDataNamePattern(page, formId, fieldName, validName, invalidName) {
  const result = await page.evaluate(`(() => {
    const form = document.getElementById(${JSON.stringify(formId)});
    const field = form?.elements.namedItem(${JSON.stringify(fieldName)});
    if (!field) return null;
    const original = field.value;
    field.value = ${JSON.stringify(validName)};
    const validAccepted = field.checkValidity();
    field.value = ${JSON.stringify(invalidName)};
    const invalidRejected = !field.checkValidity();
    field.value = original;
    return { pattern: field.pattern, validAccepted, invalidRejected };
  })()`);
  if (!result?.validAccepted || !result.invalidRejected) {
    throw new Error(`${formId}.${fieldName} did not enforce its native pattern (${JSON.stringify(result)})`);
  }
  return result;
}

async function exerciseDataWorkflow(page) {
  const validSql = "SELECT gpu_model, asking_usd_micros_per_hour, available_count FROM dataset ORDER BY gpu_model";
  const invalidSql = "SELECT * FROM hyperai_browser_unknown_table";
  const modelId = "browser-query-model";
  const viewId = "browser-query-table";
  await page.send("Emulation.setDeviceMetricsOverride", {
    width: 1440,
    height: 1000,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await page.evaluate('document.querySelector(".primary-nav [data-page=data]")?.click()');
  const seedButton = await waitForBrowserValue(page,
    'Boolean(document.querySelector(\'[data-action="seed-synthetic-data"]\'))',
    "empty local Data catalog", (value) => value === true);
  if (!seedButton) throw new Error("the empty synthetic catalog action did not render");
  await page.evaluate('document.querySelector(\'[data-action="seed-synthetic-data"]\')?.click()');
  await waitForBrowserValue(page,
    'Boolean(document.querySelector("#data-dataset-id") && document.querySelector("#data-query-form"))',
    "seeded local Data catalog", (value) => value === true);

  await submitDataQuery(page, validSql, "Query complete:");
  const firstResult = await readDataWorkflowState(page);
  if (firstResult.rowCount !== 3 || !/^[0-9a-f]{64}$/.test(firstResult.snapshotDigest)
    || !firstResult.datasetId || !firstResult.source
    || firstResult.queryStatusRole !== "status" || firstResult.queryStatusLive !== "polite"
    || !firstResult.queryStatusFocus) {
    throw new Error(`the successful Data query lost its bounded rows, provenance, or focus (${JSON.stringify(firstResult)})`);
  }

  const pipeline = await waitForBrowserValue(page,
    `(() => {
      const section = document.querySelector(".data-pipeline");
      if (!section || !section.querySelector(".data-pipeline-table tbody tr")) return null;
      const articles = Array.from(section.querySelectorAll(".data-pipeline-line article"));
      const table = section.querySelector(".data-pipeline-table");
      return {
        heading: section.querySelector("h2")?.textContent.trim() || "",
        labels: Array.from(section.querySelectorAll(".data-layer-label"), (element) => element.textContent.trim()),
        sourceRows: articles[0]?.querySelector("strong")?.textContent.trim() || "",
        sourceLabel: articles[0]?.querySelector("p")?.textContent.trim() || "",
        inputDigest: articles[0]?.querySelector("code")?.title || "",
        normalizedRows: articles[1]?.querySelector("strong")?.textContent.trim() || "",
        methodVersion: articles[1]?.querySelector("span:last-child")?.textContent.trim() || "",
        groups: articles[2]?.querySelector("strong")?.textContent.trim() || "",
        outputDigest: articles[2]?.querySelector("code")?.title || "",
        tableRows: table?.querySelectorAll("tbody tr").length || 0,
        unknownAvailability: Array.from(section.querySelectorAll(".data-null"), (element) => element.textContent.trim()),
        tableName: section.querySelector('[role="region"]')?.getAttribute("aria-label") || "",
        claim: section.querySelector(".data-claim")?.textContent.trim() || "",
        noLiveMarketClaim: Boolean(section.querySelector(".tag-caution")?.textContent.includes("No live market claim")),
      };
    })()`,
    "source-bound Bronze/Silver/Gold projection",
    (value) => value?.heading === "Bronze to Silver to Gold" && value.tableRows === 3);
  if (pipeline.labels.join("|") !== "BRONZE · VERIFIED INPUT|SILVER · NORMALIZED|GOLD · GROUP SUMMARY"
    || pipeline.sourceRows !== "3 source rows" || !pipeline.sourceLabel
    || pipeline.inputDigest !== firstResult.snapshotDigest
    || pipeline.normalizedRows !== "3 observations" || !pipeline.methodVersion
    || pipeline.groups !== "3 model/region groups"
    || !/^[0-9a-f]{64}$/.test(pipeline.outputDigest)
    || pipeline.unknownAvailability.length !== 1 || pipeline.unknownAvailability[0] !== "Unknown"
    || pipeline.tableName !== "Derived market summary table"
    || !pipeline.claim.toLowerCase().includes("synthetic observation derivation only")
    || !pipeline.claim.toLowerCase().includes("not live market data")
    || !pipeline.noLiveMarketClaim) {
    throw new Error(`the Data workspace lost derivation provenance, bounded rows, unknowns, or claim labels (${JSON.stringify(pipeline)})`);
  }

  const failureStatus = await submitDataQuery(page, invalidSql, "Query failed:");
  const failedResult = await page.evaluate(`(() => ({
    sql: document.getElementById("data-sql")?.value || "",
    resultVisible: Boolean(document.querySelector(".data-result")),
    statusRole: document.getElementById("data-query-status")?.getAttribute("role") || "",
    statusLive: document.getElementById("data-query-status")?.getAttribute("aria-live") || "",
    statusFocused: document.activeElement?.id === "data-query-status",
  }))()`);
  if (failureStatus.role !== "status" || failureStatus.live !== "polite"
    || failureStatus.focusTarget !== "data-query-status"
    || !failureStatus.text.includes("(HTTP 422)")
    || failedResult.sql !== invalidSql || failedResult.resultVisible
    || failedResult.statusRole !== "status" || failedResult.statusLive !== "polite"
    || !failedResult.statusFocused) {
    throw new Error(`the rejected Data query retained stale rows or lost its accessible failure state (${JSON.stringify({ failureStatus, failedResult })})`);
  }

  await submitDataQuery(page, validSql, "Query complete:");
  const recoveredResult = await readDataWorkflowState(page);
  if (recoveredResult.rowCount !== 3 || recoveredResult.datasetId !== firstResult.datasetId
    || recoveredResult.snapshotDigest !== firstResult.snapshotDigest || !recoveredResult.queryStatusFocus) {
    throw new Error(`the Data query did not recover against its original snapshot (${JSON.stringify(recoveredResult)})`);
  }

  const modelPattern = await assertDataNamePattern(
    page,
    "data-model-form",
    "model_id",
    modelId,
    "browser query model",
  );
  if (!await submitDataForm(page, "data-model-form", { model_id: modelId })) {
    throw new Error("the Data model preview form was not available");
  }
  await waitForBrowserValue(page,
    'document.getElementById("data-model-status")?.textContent.trim() || ""',
    "model preview status", (value) => value.startsWith("Preview ready."));
  const modelPreview = await page.evaluate(`(() => ({
    digest: document.querySelector(".data-preview-digest code")?.textContent.trim() || "",
    focused: document.activeElement?.id === "data-model-status",
    confirm: Boolean(document.querySelector('[data-action="confirm-data-model"]')),
  }))()`);
  if (!/^[0-9a-f]{64}$/.test(modelPreview.digest) || !modelPreview.focused || !modelPreview.confirm) {
    throw new Error(`the model preview was not digest-bound and confirmable (${JSON.stringify(modelPreview)})`);
  }
  await page.evaluate('document.querySelector(\'[data-action="confirm-data-model"]\')?.click()');
  const modelStatus = await waitForBrowserValue(page,
    'document.getElementById("data-model-status")?.textContent.trim() || ""',
    "model save status", (value) => value.startsWith("Saved model "));
  const modelDigest = modelStatus.match(/Definition digest ([0-9a-f]{64})\./)?.[1] || "";
  const modelListed = await page.evaluate(`Array.from(document.querySelectorAll(".data-definition-row strong"))
    .some((element) => element.textContent.trim() === ${JSON.stringify(modelId)})`);
  if (!/^[0-9a-f]{64}$/.test(modelDigest) || !modelListed) {
    throw new Error("the explicitly saved Data model was not listed with its definition digest");
  }

  const viewPattern = await assertDataNamePattern(
    page,
    "data-view-form",
    "view_id",
    viewId,
    "browser/query/table",
  );
  if (!await submitDataForm(page, "data-view-form", {
    view_id: viewId,
    presentation: "table",
    columns: "gpu_model, asking_usd_micros_per_hour",
  })) {
    throw new Error("the Data view preview form was not available after saving its model");
  }
  await waitForBrowserValue(page,
    'document.getElementById("data-view-status")?.textContent.trim() || ""',
    "view preview status", (value) => value.startsWith("Preview ready."));
  const viewPreview = await page.evaluate(`(() => ({
    digest: document.querySelector(".data-preview-digest code")?.textContent.trim() || "",
    focused: document.activeElement?.id === "data-view-status",
    confirm: Boolean(document.querySelector('[data-action="confirm-data-view"]')),
  }))()`);
  if (!/^[0-9a-f]{64}$/.test(viewPreview.digest) || !viewPreview.focused || !viewPreview.confirm) {
    throw new Error(`the view preview was not digest-bound and confirmable (${JSON.stringify(viewPreview)})`);
  }
  await page.evaluate('document.querySelector(\'[data-action="confirm-data-view"]\')?.click()');
  const viewStatus = await waitForBrowserValue(page,
    'document.getElementById("data-view-status")?.textContent.trim() || ""',
    "view save status", (value) => value.startsWith("Saved view "));
  const viewDigest = viewStatus.match(/Definition digest ([0-9a-f]{64})\./)?.[1] || "";
  const viewListed = await page.evaluate(`Array.from(document.querySelectorAll(".data-definition-row strong"))
    .some((element) => element.textContent.trim() === ${JSON.stringify(viewId)})`);
  if (!/^[0-9a-f]{64}$/.test(viewDigest) || !viewListed) {
    throw new Error("the explicitly saved Data view was not listed with its definition digest");
  }

  await page.evaluate('window.scrollTo(0, document.querySelector(".data-save-grid")?.offsetTop || 0)');
  await delay(120);
  return {
    status: "passed",
    datasetId: recoveredResult.datasetId,
    snapshotDigest: recoveredResult.snapshotDigest,
    queryRows: recoveredResult.rowCount,
    failedQueryClearedRows: !failedResult.resultVisible,
    queryRecovery: recoveredResult.rowCount === 3,
    derivedPipeline: {
      labels: pipeline.labels,
      sourceRows: pipeline.sourceRows,
      sourceLabel: pipeline.sourceLabel,
      inputDigest: pipeline.inputDigest,
      inputDigestMatchesQuery: pipeline.inputDigest === firstResult.snapshotDigest,
      normalizedRows: pipeline.normalizedRows,
      methodVersion: pipeline.methodVersion,
      groups: pipeline.groups,
      outputDigest: pipeline.outputDigest,
      tableRows: pipeline.tableRows,
      unknownAvailability: pipeline.unknownAvailability,
      tableName: pipeline.tableName,
      syntheticOnlyClaimVisible: pipeline.noLiveMarketClaim
        && pipeline.claim.toLowerCase().includes("synthetic observation derivation only")
        && pipeline.claim.toLowerCase().includes("not live market data"),
    },
    modelId,
    modelPreviewDigest: modelPreview.digest,
    modelDefinitionDigest: modelDigest,
    modelPattern: modelPattern.pattern,
    modelPatternValidAndRejectsInvalidName: modelPattern.validAccepted && modelPattern.invalidRejected,
    modelListed,
    viewId,
    viewPreviewDigest: viewPreview.digest,
    viewDefinitionDigest: viewDigest,
    viewPattern: viewPattern.pattern,
    viewPatternValidAndRejectsInvalidName: viewPattern.validAccepted && viewPattern.invalidRejected,
    viewListed,
    statusFocusRestored: firstResult.queryStatusFocus && failedResult.statusFocused
      && recoveredResult.queryStatusFocus && modelPreview.focused && viewPreview.focused,
  };
}

async function exerciseGpudiffScheduleWorkflow(page) {
  await page.send("Emulation.setDeviceMetricsOverride", {
    width: 1440,
    height: 1000,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await page.evaluate('document.querySelector(".primary-nav [data-page=data]")?.click()');
  const initial = await waitForBrowserValue(page, `(() => {
    const panel = document.querySelector(".data-schedule");
    return panel ? {
      state: panel.querySelector(".tag")?.textContent.trim() || "",
      startDisabled: panel.querySelector('[data-action="start-gpudiff-schedule"]')?.disabled ?? true,
      stopDisabled: panel.querySelector('[data-action="stop-gpudiff-schedule"]')?.disabled ?? true,
      copy: panel.textContent,
    } : null;
  })()`, "owner GPUDiff schedule controls", (value) => value?.state === "stopped");
  if (initial.startDisabled || !initial.stopDisabled
    || !initial.copy.includes("six hours")
    || !initial.copy.includes("16 distinct captures")) {
    throw new Error(`the stopped GPUDiff schedule controls or limits were not visible (${JSON.stringify(initial)})`);
  }

  await page.evaluate('document.querySelector(\'[data-action="start-gpudiff-schedule"]\')?.click()');
  const activeState = await waitForBrowserValue(
    page,
    'document.querySelector(".data-schedule .tag")?.textContent.trim() || ""',
    "active GPUDiff schedule state",
    (value) => value === "active",
    30000,
  );
  const queryStatus = await waitForBrowserValue(
    page,
    'document.getElementById("data-query-status")?.textContent.trim() || ""',
    "scheduled GPUDiff catalog query",
    (value) => value.startsWith("Query complete:"),
    30000,
  );
  const active = await page.evaluate(`(() => {
    const result = document.querySelector(".data-result");
    const rows = result ? Array.from(result.querySelectorAll(".data-result-table tbody tr")) : [];
    return {
      state: document.querySelector(".data-schedule .tag")?.textContent.trim() || "",
      datasetId: result?.querySelector(".data-result-meta code")?.textContent.trim() || "",
      digest: result?.querySelectorAll(".data-result-meta code")[1]?.title || "",
      rowCount: rows.length,
      sourceIndicator: document.getElementById("data-source-indicator")?.textContent.trim() || "",
      attribution: document.querySelector(".data-schedule-meta")?.textContent.includes("CC BY 4.0") || false,
      queryStatusRole: document.getElementById("data-query-status")?.getAttribute("role") || "",
    };
  })()`);
  if (active.state !== "active"
    || active.datasetId !== "hyperai.gpudiff_public_gpu_offers.v1"
    || !/^[0-9a-f]{64}$/.test(active.digest)
    || active.rowCount < 1
    || active.sourceIndicator !== "LOCAL · GPUDIFF SNAPSHOT"
    || !active.attribution
    || active.queryStatusRole !== "status") {
    throw new Error(`the real GPUDiff snapshot did not reach the bounded Data query UI (${JSON.stringify(active)})`);
  }

  await page.evaluate('document.querySelector(\'[data-action="stop-gpudiff-schedule"]\')?.click()');
  const stoppedState = await waitForBrowserValue(
    page,
    'document.querySelector(".data-schedule .tag")?.textContent.trim() || ""',
    "stopped GPUDiff schedule state",
    (value) => value === "stopped",
  );
  return {
    status: "passed",
    initialState: initial.state,
    activeState,
    stoppedState,
    datasetId: active.datasetId,
    normalizedDigest: active.digest,
    queryRows: active.rowCount,
    queryStatus,
    sourceIndicator: active.sourceIndicator,
    attributionVisible: active.attribution,
  };
}

async function exerciseSessionActivityOwnerLinksWorkflow(page, networkRequests, networkResponses) {
  const validSql = "SELECT gpu_model, asking_usd_micros_per_hour, available_count FROM dataset ORDER BY gpu_model";
  await page.send("Emulation.setDeviceMetricsOverride", {
    width: 1440,
    height: 1000,
    deviceScaleFactor: 1,
    mobile: false,
  });
  const actionRequestStart = networkRequests.length;
  const actionResponseStart = networkResponses.length;
  await page.evaluate('document.querySelector(".primary-nav [data-page=data]")?.click()');
  await waitForBrowserValue(page,
    'Boolean(document.querySelector(\'[data-action="seed-synthetic-data"]\') || (document.getElementById("data-dataset-id") && document.getElementById("data-query-form")))',
    "Data query or synthetic seed control", (value) => value === true);
  const dataSeedButton = await page.evaluate('Boolean(document.querySelector(\'[data-action="seed-synthetic-data"]\'))');
  if (dataSeedButton) {
    await page.evaluate('document.querySelector(\'[data-action="seed-synthetic-data"]\')?.click()');
  }
  await waitForBrowserValue(page,
    'Boolean(document.getElementById("data-dataset-id") && document.getElementById("data-query-form"))',
    "ready local Data query form", (value) => value === true);
  await submitDataQuery(page, validSql, "Query complete:");
  const dataResult = await readDataWorkflowState(page);
  if (!/^[0-9a-f]{64}$/.test(dataResult.snapshotDigest) || dataResult.rowCount !== 3) {
    throw new Error(`the local Data result lacked its fixed source snapshot identity (${JSON.stringify(dataResult)})`);
  }

  await page.evaluate('document.querySelector(".primary-nav [data-page=fleet]")?.click()');
  await waitForBrowserValue(page,
    'Boolean(document.getElementById("fleet-plan-form"))',
    "Fleet advisory preview form", (value) => value === true);
  const fleetFormSubmitted = await page.evaluate('(() => { const form = document.getElementById("fleet-plan-form"); if (!form) return false; form.requestSubmit(); return true; })()');
  if (!fleetFormSubmitted) throw new Error("the Fleet advisory preview form was not available");
  const fleetPlan = await waitForBrowserValue(page,
    `(() => { const section = document.querySelector(".fleet-plan-result"); const digest = section?.querySelector(".fleet-plan-status code")?.title || ""; return /^[0-9a-f]{64}$/.test(digest) ? { digest, title: section.querySelector("h3")?.textContent.trim() || "" } : null; })()`,
    "digest-bound Fleet preview", (value) => value !== null);

  await page.evaluate('document.querySelector(".primary-nav [data-page=eval]")?.click()');
  await waitForBrowserValue(page,
    'Boolean(document.querySelector(\'[data-action="run-eval"]:not(:disabled)\'))',
    "bounded Eval fixture run control", (value) => value === true);
  await page.evaluate('document.querySelector(\'[data-action="run-eval"]\')?.click()');
  const evalResult = await waitForBrowserValue(page,
    `(() => { const report = document.querySelector(".eval-report"); const terms = Array.from(report?.querySelectorAll(".detail-list dt") || []); const suite = terms.find((term) => term.textContent.trim() === "Suite digest")?.nextElementSibling?.querySelector("code")?.textContent.trim() || ""; const repeated = terms.find((term) => term.textContent.trim() === "Repeated result digest")?.nextElementSibling?.querySelector("code")?.textContent.trim() || ""; return /^[0-9a-f]{64}$/.test(suite) && /^[0-9a-f]{64}$/.test(repeated) ? { suite, repeated } : null; })()`,
    "fixed Eval fixture report", (value) => value !== null);

  await page.evaluate('document.querySelector(".primary-nav [data-page=trade]")?.click()');
  await waitForBrowserValue(page,
    'Boolean(document.getElementById("trade-form"))',
    "Trade paper calculation form", (value) => value === true);
  const tradeFormSubmitted = await page.evaluate('(() => { const form = document.getElementById("trade-form"); if (!form) return false; form.requestSubmit(); return true; })()');
  if (!tradeFormSubmitted) throw new Error("the Trade paper calculation form was not available");
  let tradeResult;
  try {
    tradeResult = await waitForBrowserValue(page,
      `(() => { const digest = document.querySelector(".trade-replay-digest code")?.textContent.trim() || ""; return /^[0-9a-f]{64}$/.test(digest) ? digest : null; })()`,
      "bounded Trade paper replay", (value) => value !== null);
  } catch (error) {
    const diagnostic = await page.evaluate(`(() => ({
      heading: document.getElementById("workspace-page-title")?.textContent.trim() || "",
      banner: document.getElementById("connection-banner")?.innerText.trim() || "",
      paperCard: Boolean(Array.from(document.querySelectorAll(".trade-result-card h2"))
        .some((element) => element.textContent.trim() === "Known-capacity replay")),
    }))()`);
    const tradeResponses = networkResponses
      .filter(({ url }) => new URL(url).pathname.startsWith("/api/v1/trade/"))
      .map(({ url, status }) => ({ path: new URL(url).pathname, status }));
    throw new Error(`Trade paper replay did not render: ${JSON.stringify({ diagnostic, tradeResponses, cause: error.message })}`);
  }

  const expectedRecords = [
    { source: "HYPERAI DATA", title: "Local query result", digest: dataResult.snapshotDigest, digestLabel: "Source snapshot digest", status: "LOCAL QUERY RESULT", page: "data" },
    { source: "HYPERAI FLEET", title: "Advisory operation preview", digest: fleetPlan.digest, digestLabel: "Preview digest", status: "ADVISORY PREVIEW", page: "fleet" },
    { source: "HYPERAI EVAL", title: "Harness fixture comparison", digest: evalResult.repeated, digestLabel: "Repeated fixture result digest", status: "SYNTHETIC FIXTURE", page: "eval" },
    { source: "HYPERAI TRADE", title: "Paper procurement-cost replay", digest: tradeResult, digestLabel: "Paper replay digest", status: "LOCAL RESEARCH", page: "trade" },
  ];
  await page.evaluate('document.querySelector(".primary-nav [data-page=activity]")?.click()');
  const activityRecords = await waitForBrowserValue(page,
    `(() => { const expected = ${JSON.stringify(expectedRecords)}; const cards = Array.from(document.querySelectorAll(".session-record")); return expected.every((item) => cards.some((card) => card.querySelector("h3")?.textContent.trim() === item.title)) ? cards.map((card) => ({ source: card.querySelector(".eyebrow")?.textContent.trim() || "", title: card.querySelector("h3")?.textContent.trim() || "", digest: card.querySelector("code")?.title || "", digestLabel: card.querySelector(".session-record-identity")?.textContent.trim() || "", status: card.querySelector(".tag")?.textContent.trim() || "", note: card.querySelector("small")?.textContent.trim() || "", page: card.querySelector("button[data-page]")?.dataset.page || "", link: card.querySelector("button[data-page]")?.textContent.trim() || "" })) : null; })()`,
    "four source-linked Activity records", (value) => Array.isArray(value) && value.length === expectedRecords.length);
  if (JSON.stringify(activityRecords.map(({ source, title, digest, digestLabel, status, page }) => ({ source, title, digest, digestLabel, status, page })))
    !== JSON.stringify(expectedRecords)) {
    throw new Error(`Activity did not preserve the exact owner result digests and labels (${JSON.stringify(activityRecords)})`);
  }
  const activityText = await page.evaluate('document.querySelector(".session-activity")?.innerText || ""');
  if (activityText.includes("SELECT gpu_model") || activityText.includes("asking_usd_micros_per_hour, available_count")) {
    throw new Error("Activity exposed the Data query text instead of a source-snapshot reference");
  }

  const navigations = [];
  for (const item of expectedRecords) {
    const requestCountBeforeNavigation = networkRequests.length;
    const clicked = await page.evaluate(`(() => { const card = Array.from(document.querySelectorAll(".session-record"))
      .find((element) => element.querySelector("h3")?.textContent.trim() === ${JSON.stringify(item.title)});
      const button = card?.querySelector('button[data-page="${item.page}"]'); if (!button) return false; button.click(); return true; })()`);
    if (!clicked) throw new Error(`Activity link to ${item.page} was not available`);
    const ownerPage = await waitForBrowserValue(page,
      `(() => { const title = document.getElementById("workspace-page-title"); return title?.textContent.trim() === ${JSON.stringify(({ data: "Data", fleet: "Fleet", eval: "Eval", trade: "Trade" })[item.page])} ? title.textContent.trim() : null; })()`,
      `${item.page} owner workspace`, (value) => value !== null);
    const ownerResult = await page.evaluate(`(() => {
      const title = document.getElementById("workspace-page-title");
      const digest = ${JSON.stringify(item.digest)};
      const page = ${JSON.stringify(item.page)};
      let ownerDigest = "";
      if (page === "data") ownerDigest = document.querySelector(".data-result-meta code[title]")?.title || "";
      if (page === "fleet") ownerDigest = document.querySelector(".fleet-plan-status code")?.title || "";
      if (page === "eval") ownerDigest = Array.from(document.querySelectorAll(".eval-report .detail-list dt"))
        .find((term) => term.textContent.trim() === "Repeated result digest")?.nextElementSibling?.querySelector("code")?.textContent.trim() || "";
      if (page === "trade") ownerDigest = document.querySelector(".trade-replay-digest code")?.textContent.trim() || "";
      return { heading: title?.textContent.trim() || "", focused: document.activeElement === title, digest: ownerDigest, matches: ownerDigest === digest };
    })()`);
    if (ownerResult.heading !== ownerPage || !ownerResult.focused || !ownerResult.matches) {
      throw new Error(`the ${item.page} link did not return to its owner result and heading (${JSON.stringify(ownerResult)})`);
    }
    if (networkRequests.length !== requestCountBeforeNavigation) {
      throw new Error(`navigation to ${item.page} issued a service request`);
    }
    navigations.push({ page: item.page, focused: ownerResult.focused, digestMatches: ownerResult.matches });
    await page.evaluate('document.querySelector(".primary-nav [data-page=activity]")?.click()');
    await waitForBrowserValue(page,
      `Array.from(document.querySelectorAll(".session-record h3"), (element) => element.textContent.trim()).includes(${JSON.stringify(item.title)})`,
      `retained Activity reference for ${item.page}`, (value) => value === true);
  }

  await page.send("Emulation.setDeviceMetricsOverride", {
    width: 390,
    height: 844,
    deviceScaleFactor: 1,
    mobile: true,
  });
  const mobile = await page.evaluate('(() => ({ width: document.documentElement.clientWidth, scrollWidth: document.documentElement.scrollWidth, cards: document.querySelectorAll(".session-record").length }))()');
  if (mobile.cards !== expectedRecords.length || mobile.scrollWidth > mobile.width) {
    throw new Error(`the populated Activity cards overflow or disappear at mobile width (${JSON.stringify(mobile)})`);
  }
  const expectedPosts = [
    "/api/v1/data/synthetic/seed",
    "/api/v1/data/query",
    "/api/v1/fleet/operation-preview",
    "/api/v1/eval/run",
    "/api/v1/trade/scenario",
    "/api/v1/trade/paper",
  ];
  const observedPosts = networkRequests.slice(actionRequestStart)
    .filter(({ method }) => method === "POST")
    .map(({ url }) => new URL(url).pathname);
  if (JSON.stringify([...observedPosts].sort()) !== JSON.stringify([...expectedPosts].sort())) {
    throw new Error(`the Activity workflow sent unexpected local mutations (${JSON.stringify(observedPosts)})`);
  }
  const postStatuses = expectedPosts.map((route) => ({
    route,
    statuses: networkResponses.slice(actionResponseStart)
      .filter(({ url }) => new URL(url).pathname === route)
      .map(({ status }) => status),
  }));
  if (postStatuses.some(({ statuses }) => JSON.stringify(statuses) !== JSON.stringify([200]))) {
    throw new Error(`an Activity workflow owner route did not return one HTTP 200 (${JSON.stringify(postStatuses)})`);
  }
  return {
    status: "passed",
    records: expectedRecords.map(({ source, title, digest, digestLabel, status, page }) => ({ source, title, digest, digestLabel, status, page })),
    ownerNavigations: navigations,
    mobile: { width: mobile.width, scrollWidth: mobile.scrollWidth, cards: mobile.cards },
    dataQueryTextOmitted: true,
    localPostRoutes: postStatuses,
    serviceRequestsBeforeActivityNavigation: networkRequests.length - actionRequestStart,
  };
}

async function exerciseDataFailureWorkflow(page, home) {
  const validSql = "SELECT gpu_model, asking_usd_micros_per_hour, available_count FROM dataset ORDER BY gpu_model";
  const alternateSql = "SELECT gpu_model, asking_usd_micros_per_hour, available_count FROM dataset ORDER BY asking_usd_micros_per_hour DESC";
  const modelId = "browser-preview-failure-model";
  const viewId = "browser-preview-failure-table";
  await page.send("Emulation.setDeviceMetricsOverride", {
    width: 1440,
    height: 1000,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await page.evaluate('document.querySelector(".primary-nav [data-page=data]")?.click()');
  const seedButton = await waitForBrowserValue(page,
    'Boolean(document.querySelector(\'[data-action="seed-synthetic-data"]\'))',
    "empty local Data catalog", (value) => value === true);
  if (!seedButton) throw new Error("the empty synthetic catalog action did not render");
  await page.evaluate('document.querySelector(\'[data-action="seed-synthetic-data"]\')?.click()');
  await waitForBrowserValue(page,
    'Boolean(document.getElementById("data-dataset-id") && document.getElementById("data-query-form"))',
    "seeded local Data catalog", (value) => value === true);

  await submitDataQuery(page, validSql, "Query complete:");
  const initialResult = await readDataWorkflowState(page);
  if (initialResult.rowCount !== 3 || !/^[0-9a-f]{64}$/.test(initialResult.snapshotDigest)
    || !initialResult.datasetId || !initialResult.source) {
    throw new Error(`the failure workflow did not start from the verified synthetic snapshot (${JSON.stringify(initialResult)})`);
  }

  const syntheticJsonPath = path.join(home, "data", "synthetic-market-observations.json");
  const originalSnapshotBytes = fs.readFileSync(syntheticJsonPath);
  let snapshotRestored = false;
  let modelFailure;
  try {
    fs.writeFileSync(syntheticJsonPath, Buffer.concat([originalSnapshotBytes, Buffer.from("\n")]));
    if (!await submitDataForm(page, "data-model-form", { model_id: modelId })) {
      throw new Error("the Data model preview form was not available for the integrity-failure case");
    }
    modelFailure = await waitForBrowserValue(page,
      `(() => {
        const status = document.getElementById("data-model-status");
        if (!status) return null;
        return {
          text: status.textContent.trim(),
          role: status.getAttribute("role"),
          live: status.getAttribute("aria-live"),
          focused: document.activeElement?.id === "data-model-status",
          modelId: document.querySelector('#data-model-form input[name="model_id"]')?.value || "",
          formVisible: Boolean(document.getElementById("data-model-form")),
          confirmVisible: Boolean(document.querySelector('[data-action="confirm-data-model"]')),
        };
      })()`,
      "integrity-rejected model preview",
      (value) => value?.text.startsWith("Preview failed:"));
  } finally {
    fs.writeFileSync(syntheticJsonPath, originalSnapshotBytes);
    snapshotRestored = fs.readFileSync(syntheticJsonPath).equals(originalSnapshotBytes);
  }
  if (!snapshotRestored) throw new Error("the disposable synthetic JSON bytes were not restored exactly");
  if (modelFailure.role !== "status" || modelFailure.live !== "polite" || !modelFailure.focused
    || modelFailure.modelId !== modelId || !modelFailure.formVisible || modelFailure.confirmVisible) {
    throw new Error(`the failed model preview lost its accessible status or editable state (${JSON.stringify(modelFailure)})`);
  }

  if (!await submitDataForm(page, "data-model-form", { model_id: modelId })) {
    throw new Error("the recovered Data model preview form was not available");
  }
  await waitForBrowserValue(page,
    'document.getElementById("data-model-status")?.textContent.trim() || ""',
    "first model preview", (value) => value.startsWith("Preview ready."));
  const firstModelPreview = await page.evaluate(`(() => ({
    digest: document.querySelector(".data-preview-digest code")?.textContent.trim() || "",
    confirm: Boolean(document.querySelector('[data-action="confirm-data-model"]')),
  }))()`);
  if (!/^[0-9a-f]{64}$/.test(firstModelPreview.digest) || !firstModelPreview.confirm) {
    throw new Error(`the first valid model preview was not digest-bound (${JSON.stringify(firstModelPreview)})`);
  }
  await submitDataQuery(page, alternateSql, "Query complete:");
  const modelPreviewCleared = await page.evaluate(`(() => ({
    confirm: Boolean(document.querySelector('[data-action="confirm-data-model"]')),
    staleStatus: Boolean(document.getElementById("data-model-status")),
    form: Boolean(document.getElementById("data-model-form")),
  }))()`);
  if (modelPreviewCleared.confirm || modelPreviewCleared.staleStatus || !modelPreviewCleared.form) {
    throw new Error(`a new query left a stale model preview visible (${JSON.stringify(modelPreviewCleared)})`);
  }

  if (!await submitDataForm(page, "data-model-form", { model_id: modelId })) {
    throw new Error("the model form was not available after clearing its stale preview");
  }
  await waitForBrowserValue(page,
    'document.getElementById("data-model-status")?.textContent.trim() || ""',
    "final model preview", (value) => value.startsWith("Preview ready."));
  const finalModelPreview = await page.evaluate(`(() => ({
    digest: document.querySelector(".data-preview-digest code")?.textContent.trim() || "",
    confirm: Boolean(document.querySelector('[data-action="confirm-data-model"]')),
  }))()`);
  if (!/^[0-9a-f]{64}$/.test(finalModelPreview.digest) || !finalModelPreview.confirm) {
    throw new Error(`the final model preview was not digest-bound (${JSON.stringify(finalModelPreview)})`);
  }
  await page.evaluate('document.querySelector(\'[data-action="confirm-data-model"]\')?.click()');
  const modelStatus = await waitForBrowserValue(page,
    'document.getElementById("data-model-status")?.textContent.trim() || ""',
    "model save status", (value) => value.startsWith("Saved model "));
  const modelDigest = modelStatus.match(/Definition digest ([0-9a-f]{64})\./)?.[1] || "";
  const modelListed = await page.evaluate(`Array.from(document.querySelectorAll(".data-definition-row strong"))
    .some((element) => element.textContent.trim() === ${JSON.stringify(modelId)})`);
  if (!/^[0-9a-f]{64}$/.test(modelDigest) || !modelListed) {
    throw new Error("the explicitly saved Data model was not listed with its definition digest");
  }

  const invalidColumns = "not a column";
  if (!await submitDataForm(page, "data-view-form", {
    view_id: viewId,
    presentation: "table",
    columns: invalidColumns,
  })) {
    throw new Error("the Data view preview form was not available for the invalid-column case");
  }
  const viewFailure = await waitForBrowserValue(page,
    `(() => {
      const status = document.getElementById("data-view-status");
      if (!status) return null;
      return {
        text: status.textContent.trim(),
        role: status.getAttribute("role"),
        live: status.getAttribute("aria-live"),
        atomic: status.getAttribute("aria-atomic"),
        focused: document.activeElement?.id === "data-view-status",
        viewId: document.querySelector('#data-view-form input[name="view_id"]')?.value || "",
        columns: document.querySelector('#data-view-form input[name="columns"]')?.value || "",
        formVisible: Boolean(document.getElementById("data-view-form")),
        confirmVisible: Boolean(document.querySelector('[data-action="confirm-data-view"]')),
      };
    })()`,
    "rejected view preview",
    (value) => value?.text.startsWith("Preview failed:"));
  if (viewFailure.role !== "status" || viewFailure.live !== "polite" || viewFailure.atomic !== "true"
    || !viewFailure.focused || viewFailure.viewId !== viewId || viewFailure.columns !== invalidColumns
    || !viewFailure.formVisible || viewFailure.confirmVisible) {
    throw new Error(`the failed view preview lost its accessible status or editable fields (${JSON.stringify(viewFailure)})`);
  }

  const validColumns = "gpu_model, asking_usd_micros_per_hour";
  if (!await submitDataForm(page, "data-view-form", {
    view_id: viewId,
    presentation: "table",
    columns: validColumns,
  })) {
    throw new Error("the Data view preview form was not available after editing its columns");
  }
  await waitForBrowserValue(page,
    'document.getElementById("data-view-status")?.textContent.trim() || ""',
    "first view preview", (value) => value.startsWith("Preview ready."));
  const firstViewPreview = await page.evaluate(`(() => ({
    digest: document.querySelector(".data-preview-digest code")?.textContent.trim() || "",
    confirm: Boolean(document.querySelector('[data-action="confirm-data-view"]')),
  }))()`);
  if (!/^[0-9a-f]{64}$/.test(firstViewPreview.digest) || !firstViewPreview.confirm) {
    throw new Error(`the first valid view preview was not digest-bound (${JSON.stringify(firstViewPreview)})`);
  }
  await submitDataQuery(page, validSql, "Query complete:");
  const viewPreviewCleared = await page.evaluate(`(() => ({
    confirm: Boolean(document.querySelector('[data-action="confirm-data-view"]')),
    staleStatus: Boolean(document.getElementById("data-view-status")),
    form: Boolean(document.getElementById("data-view-form")),
    modelStillSaved: Array.from(document.querySelectorAll(".data-definition-row strong"))
      .some((element) => element.textContent.trim() === ${JSON.stringify(modelId)}),
  }))()`);
  if (viewPreviewCleared.confirm || viewPreviewCleared.staleStatus || !viewPreviewCleared.form
    || !viewPreviewCleared.modelStillSaved) {
    throw new Error(`a new query left a stale view preview visible or cleared the saved model (${JSON.stringify(viewPreviewCleared)})`);
  }

  if (!await submitDataForm(page, "data-view-form", {
    view_id: viewId,
    presentation: "table",
    columns: validColumns,
  })) {
    throw new Error("the view form was not available after clearing its stale preview");
  }
  await waitForBrowserValue(page,
    'document.getElementById("data-view-status")?.textContent.trim() || ""',
    "final view preview", (value) => value.startsWith("Preview ready."));
  const finalViewPreview = await page.evaluate(`(() => ({
    digest: document.querySelector(".data-preview-digest code")?.textContent.trim() || "",
    confirm: Boolean(document.querySelector('[data-action="confirm-data-view"]')),
  }))()`);
  if (!/^[0-9a-f]{64}$/.test(finalViewPreview.digest) || !finalViewPreview.confirm) {
    throw new Error(`the final view preview was not digest-bound (${JSON.stringify(finalViewPreview)})`);
  }
  await page.evaluate('document.querySelector(\'[data-action="confirm-data-view"]\')?.click()');
  const viewStatus = await waitForBrowserValue(page,
    'document.getElementById("data-view-status")?.textContent.trim() || ""',
    "view save status", (value) => value.startsWith("Saved view "));
  const viewDigest = viewStatus.match(/Definition digest ([0-9a-f]{64})\./)?.[1] || "";
  const viewListed = await page.evaluate(`Array.from(document.querySelectorAll(".data-definition-row strong"))
    .some((element) => element.textContent.trim() === ${JSON.stringify(viewId)})`);
  if (!/^[0-9a-f]{64}$/.test(viewDigest) || !viewListed) {
    throw new Error("the explicitly saved Data view was not listed with its definition digest");
  }

  return {
    status: "passed",
    datasetId: initialResult.datasetId,
    snapshotDigest: initialResult.snapshotDigest,
    modelId,
    modelIntegrityFailureStatus: 409,
    modelFailureStatusFocused: modelFailure.focused,
    modelPreviewClearedAfterQuery: !modelPreviewCleared.confirm && !modelPreviewCleared.staleStatus,
    modelPreviewDigests: [firstModelPreview.digest, finalModelPreview.digest],
    modelDefinitionDigest: modelDigest,
    modelListed,
    viewId,
    viewValidationFailureStatus: 422,
    viewFailureStatusFocused: viewFailure.focused,
    viewFailureFieldsRetained: viewFailure.viewId === viewId && viewFailure.columns === invalidColumns,
    viewPreviewClearedAfterQuery: !viewPreviewCleared.confirm && !viewPreviewCleared.staleStatus,
    viewPreviewDigests: [firstViewPreview.digest, finalViewPreview.digest],
    viewDefinitionDigest: viewDigest,
    viewListed,
    syntheticSnapshotRestored: snapshotRestored,
  };
}

async function dispatchKeyboardKey(page, key, code, windowsVirtualKeyCode, modifiers = 0) {
  const event = { key, code, windowsVirtualKeyCode, modifiers };
  await page.send("Input.dispatchKeyEvent", { type: "rawKeyDown", ...event });
  await page.send("Input.dispatchKeyEvent", { type: "keyUp", ...event });
}

async function dispatchKeyboardEnter(page) {
  const event = {
    key: "Enter",
    code: "Enter",
    windowsVirtualKeyCode: 13,
    text: "\r",
    unmodifiedText: "\r",
  };
  await page.send("Input.dispatchKeyEvent", { type: "rawKeyDown", ...event, text: "", unmodifiedText: "" });
  await page.send("Input.dispatchKeyEvent", { type: "char", ...event });
  await page.send("Input.dispatchKeyEvent", { type: "keyUp", ...event, text: "", unmodifiedText: "" });
}

async function dispatchKeyboardText(page, value) {
  const virtualKeys = new Map([[" ", 32], [",", 188], ["-", 189], ["_", 189]]);
  const physicalKeys = new Map([[" ", "Space"], [",", "Comma"], ["-", "Minus"], ["_", "Minus"]]);
  for (const character of value) {
    const letter = /^[a-z]$/i.test(character);
    const event = {
      key: character,
      code: physicalKeys.get(character) || (letter ? `Key${character.toUpperCase()}` : ""),
      windowsVirtualKeyCode: virtualKeys.get(character) || (letter ? character.toUpperCase().charCodeAt(0) : character.charCodeAt(0)),
      text: character,
      unmodifiedText: character,
    };
    await page.send("Input.dispatchKeyEvent", {
      type: "rawKeyDown",
      ...event,
      text: "",
      unmodifiedText: "",
    });
    await page.send("Input.dispatchKeyEvent", { type: "char", ...event });
    await page.send("Input.dispatchKeyEvent", { type: "keyUp", ...event });
  }
}

async function tabToControl(page, selector, description, limit = 60) {
  for (let index = 0; index <= limit; index += 1) {
    const focused = await page.evaluate(`Boolean(document.activeElement?.matches(${JSON.stringify(selector)}))`);
    if (focused) return index;
    await dispatchKeyboardKey(page, "Tab", "Tab", 9);
  }
  const active = await page.evaluate(`(() => {
    const element = document.activeElement;
    return element ? {
      tag: element.tagName,
      id: element.id,
      name: element.getAttribute("name"),
      text: (element.innerText || element.textContent || "").trim().slice(0, 80),
    } : null;
  })()`);
  throw new Error(`keyboard Tab navigation did not reach ${description} (${JSON.stringify(active)})`);
}

async function readComputedAccessibleNode(page, selector) {
  const document = await page.send("DOM.getDocument", { depth: 1 });
  const queried = await page.send("DOM.querySelector", {
    nodeId: document.root.nodeId,
    selector,
  });
  if (!queried.nodeId) {
    throw new Error(`the accessible control is absent for selector ${selector}`);
  }
  const tree = await page.send("Accessibility.getPartialAXTree", {
    nodeId: queried.nodeId,
    fetchRelatives: false,
  });
  const node = tree.nodes?.find((candidate) => !candidate.ignored);
  if (!node) return { present: false, role: "", name: "", ignored: true };
  return {
    present: true,
    role: node.role?.value || "",
    name: node.name?.value || "",
    description: node.description?.value || "",
    ignored: Boolean(node.ignored),
  };
}

function normalizeAccessibleNameForPresentation(value) {
  return String(value)
    .normalize("NFC")
    .replace(/\p{White_Space}+/gu, " ")
    .trim()
    .replace(/[A-Z]/g, (character) => character.toLowerCase());
}

async function assertComputedAccessibleNode(page, selector, description, expectedRole, expectedName, nameComparison = "exact") {
  const accessible = await readComputedAccessibleNode(page, selector);
  const nameMatches = nameComparison === "exact"
    ? accessible.name === expectedName
    : nameComparison === "nfc-whitespace-ascii-case-insensitive"
      ? normalizeAccessibleNameForPresentation(accessible.name) === normalizeAccessibleNameForPresentation(expectedName)
      : false;
  if (!accessible.present || accessible.ignored
    || accessible.role !== expectedRole || !nameMatches) {
    throw new Error(`Chromium computed an unexpected accessible role/name for ${description} (${JSON.stringify({
      expected: { role: expectedRole, name: expectedName, comparison: nameComparison },
      actual: accessible,
    })})`);
  }
  return accessible;
}

async function assertKeyboardFocusVisible(page, selector, description, expectedRole, expectedName, nameComparison = "exact") {
  await tabToControl(page, selector, description);
  const state = await page.evaluate(`(() => {
    const element = document.querySelector(${JSON.stringify(selector)});
    const active = document.activeElement;
    if (!element || element !== active) return null;
    const style = getComputedStyle(element);
    const outlineVisible = style.outlineStyle !== "none" && Number.parseFloat(style.outlineWidth) > 0;
    const shadowVisible = style.boxShadow !== "none";
    return {
      tag: element.tagName,
      focusVisible: element.matches(":focus-visible"),
      indicatorVisible: outlineVisible || shadowVisible,
      outline: style.outline,
    };
  })()`);
  const accessible = await assertComputedAccessibleNode(page, selector, description, expectedRole, expectedName, nameComparison);
  if (!state?.focusVisible || !state.indicatorVisible) {
    throw new Error(`keyboard focus was not visibly indicated for ${description} (${JSON.stringify(state)})`);
  }
  return { ...state, accessibleRole: accessible.role, accessibleName: accessible.name };
}

async function replaceFocusedTextByKeyboard(page, selector, value, description, expectedRole = null, expectedName = null, nameComparison = "exact") {
  await tabToControl(page, selector, description);
  let accessible = null;
  if (expectedRole !== null || expectedName !== null) {
    if (expectedRole === null || expectedName === null) {
      throw new Error(`incomplete accessible-role/name expectation for ${description}`);
    }
    accessible = await assertComputedAccessibleNode(page, selector, description, expectedRole, expectedName, nameComparison);
  }
  const selectAllModifier = process.platform === "darwin" ? 4 : 2;
  await dispatchKeyboardKey(page, "a", "KeyA", 65, selectAllModifier);
  await dispatchKeyboardText(page, value);
  const result = await page.evaluate(`(() => {
    const element = document.activeElement;
    return {
      focused: Boolean(element?.matches(${JSON.stringify(selector)})),
      value: element?.value || "",
    };
  })()`);
  if (!result.focused || result.value !== value) {
    throw new Error(`keyboard editing did not retain ${description} (${JSON.stringify(result)})`);
  }
  return accessible;
}

async function replaceSingleLineTextareaByKeyboard(page, selector, value, description) {
  const original = await page.evaluate(`(() => {
    const element = document.activeElement;
    return element instanceof HTMLTextAreaElement && element.matches(${JSON.stringify(selector)})
      ? element.value
      : null;
  })()`);
  if (typeof original !== "string") {
    throw new Error(`keyboard editing did not focus the ${description} textarea`);
  }
  await dispatchKeyboardKey(page, "Home", "Home", 36);
  await dispatchKeyboardKey(page, "End", "End", 35, 8);
  const selection = await page.evaluate(`(() => {
    const element = document.activeElement;
    return element instanceof HTMLTextAreaElement && element.matches(${JSON.stringify(selector)})
      ? { start: element.selectionStart, end: element.selectionEnd, length: element.value.length }
      : null;
  })()`);
  if (selection?.start !== 0 || selection.end !== selection.length || selection.length !== original.length) {
    throw new Error(`keyboard range selection did not cover the full ${description} (${JSON.stringify(selection)})`);
  }
  await dispatchKeyboardText(page, value);
  const result = await page.evaluate(`(() => {
    const element = document.activeElement;
    return {
      focused: element instanceof HTMLTextAreaElement && element.matches(${JSON.stringify(selector)}),
      value: element instanceof HTMLTextAreaElement ? element.value : "",
    };
  })()`);
  if (!result.focused || result.value !== value) {
    throw new Error(`keyboard editing did not retain ${description} (${JSON.stringify(result)})`);
  }
}

async function assertKeyboardSubmitReady(page, formSelector, buttonSelector, description) {
  const result = await page.evaluate(`(() => {
    const form = document.querySelector(${JSON.stringify(formSelector)});
    const button = document.querySelector(${JSON.stringify(buttonSelector)});
    return {
      buttonFocused: document.activeElement === button,
      buttonDisabled: Boolean(button?.disabled),
      formValid: Boolean(form?.checkValidity()),
      invalidFields: form ? Array.from(form.elements)
        .filter((element) => element.willValidate && !element.validity.valid)
        .map((element) => ({ name: element.name, value: element.value, message: element.validationMessage })) : [],
    };
  })()`);
  if (!result.buttonFocused || result.buttonDisabled || !result.formValid) {
    throw new Error(`keyboard ${description} was blocked before activation (${JSON.stringify(result)})`);
  }
}

async function exerciseDataKeyboardViewWorkflow(page) {
  const validSql = "SELECT gpu_model, asking_usd_micros_per_hour, available_count FROM dataset ORDER BY gpu_model";
  const modelId = "keyboard-view-prerequisite-model";
  const viewId = "keyboard-view-failure-recovery";
  const invalidColumns = "not a column";
  const validColumns = "gpu_model, asking_usd_micros_per_hour";
  await page.send("Emulation.setDeviceMetricsOverride", {
    width: 1440,
    height: 1000,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await page.evaluate('document.querySelector(".primary-nav [data-page=data]")?.click()');
  const seedButton = await waitForBrowserValue(page,
    'Boolean(document.querySelector(\'[data-action="seed-synthetic-data"]\'))',
    "empty local Data catalog", (value) => value === true);
  if (!seedButton) throw new Error("the empty synthetic catalog action did not render");
  await page.evaluate('document.querySelector(\'[data-action="seed-synthetic-data"]\')?.click()');
  await waitForBrowserValue(page,
    'Boolean(document.getElementById("data-dataset-id") && document.getElementById("data-query-form"))',
    "seeded local Data catalog", (value) => value === true);
  await submitDataQuery(page, validSql, "Query complete:");

  if (!await submitDataForm(page, "data-model-form", { model_id: modelId })) {
    throw new Error("the keyboard workflow could not prepare its saved-model prerequisite");
  }
  await waitForBrowserValue(page,
    'document.getElementById("data-model-status")?.textContent.trim() || ""',
    "model prerequisite preview", (value) => value.startsWith("Preview ready."));
  const modelConfirmation = await page.evaluate(`(() => ({
    digest: document.querySelector(".data-preview-digest code")?.textContent.trim() || "",
    confirm: Boolean(document.querySelector('[data-action="confirm-data-model"]')),
  }))()`);
  if (!/^[0-9a-f]{64}$/.test(modelConfirmation.digest) || !modelConfirmation.confirm) {
    throw new Error(`the keyboard workflow model prerequisite was not digest-bound (${JSON.stringify(modelConfirmation)})`);
  }
  await page.evaluate('document.querySelector(\'[data-action="confirm-data-model"]\')?.click()');
  await waitForBrowserValue(page,
    'document.getElementById("data-model-status")?.textContent.trim() || ""',
    "saved model prerequisite", (value) => value.startsWith("Saved model "));
  await waitForBrowserValue(page,
    'Boolean(document.querySelector("#data-view-form input[name=columns]"))',
    "rendered Data view form", (value) => value === true);

  await replaceFocusedTextByKeyboard(page, '#data-view-form input[name="view_id"]', viewId, "view name");
  const columnsSelector = '#data-view-form input[name="columns"]';
  const submitSelector = '#data-view-form button[type="submit"]';
  await replaceFocusedTextByKeyboard(page, columnsSelector, invalidColumns, "invalid view columns");
  await tabToControl(page, submitSelector, "Preview view button");
  await assertKeyboardSubmitReady(page, "#data-view-form", submitSelector, "Preview view");
  await dispatchKeyboardEnter(page);
  const failure = await waitForBrowserValue(page,
    `(() => {
      const status = document.getElementById("data-view-status");
      if (!status) return null;
      return {
        text: status.textContent.trim(),
        role: status.getAttribute("role"),
        live: status.getAttribute("aria-live"),
        atomic: status.getAttribute("aria-atomic"),
        focused: document.activeElement?.id === "data-view-status",
        viewId: document.querySelector('#data-view-form input[name="view_id"]')?.value || "",
        columns: document.querySelector(${JSON.stringify(columnsSelector)})?.value || "",
        formVisible: Boolean(document.getElementById("data-view-form")),
        confirmVisible: Boolean(document.querySelector('[data-action="confirm-data-view"]')),
      };
    })()`,
    "keyboard-rejected view preview",
    (value) => value?.text.startsWith("Preview failed:"));
  if (failure.role !== "status" || failure.live !== "polite" || failure.atomic !== "true"
    || !failure.focused || failure.viewId !== viewId || failure.columns !== invalidColumns
    || !failure.formVisible || failure.confirmVisible) {
    throw new Error(`keyboard rejection lost accessible status, focus, or editable values (${JSON.stringify(failure)})`);
  }

  await replaceFocusedTextByKeyboard(page, columnsSelector, validColumns, "corrected view columns");
  await tabToControl(page, submitSelector, "corrected Preview view button");
  await assertKeyboardSubmitReady(page, "#data-view-form", submitSelector, "corrected Preview view");
  await dispatchKeyboardEnter(page);
  await waitForBrowserValue(page,
    'document.getElementById("data-view-status")?.textContent.trim() || ""',
    "keyboard-corrected view preview", (value) => value.startsWith("Preview ready."));
  const preview = await page.evaluate(`(() => ({
    digest: document.querySelector(".data-preview-digest code")?.textContent.trim() || "",
    confirm: Boolean(document.querySelector('[data-action="confirm-data-view"]')),
    focusedStatus: document.activeElement?.id === "data-view-status",
  }))()`);
  if (!/^[0-9a-f]{64}$/.test(preview.digest) || !preview.confirm || !preview.focusedStatus) {
    throw new Error(`keyboard-corrected view preview was not digest-bound or focused (${JSON.stringify(preview)})`);
  }

  await tabToControl(page, '[data-action="confirm-data-view"]', "Confirm view save button");
  await dispatchKeyboardEnter(page);
  const savedStatus = await waitForBrowserValue(page,
    'document.getElementById("data-view-status")?.textContent.trim() || ""',
    "keyboard-confirmed view save", (value) => value.startsWith("Saved view "));
  const definitionDigest = savedStatus.match(/Definition digest ([0-9a-f]{64})\./)?.[1] || "";
  const viewListed = await page.evaluate(`Array.from(document.querySelectorAll(".data-definition-row strong"))
    .some((element) => element.textContent.trim() === ${JSON.stringify(viewId)})`);
  if (!/^[0-9a-f]{64}$/.test(definitionDigest) || !viewListed) {
    throw new Error("keyboard confirmation did not save and list the view definition digest");
  }
  return {
    status: "passed",
    modelId,
    modelDigest: modelConfirmation.digest,
    viewId,
    invalidColumnsRetained: failure.columns === invalidColumns,
    failureStatusFocused: failure.focused,
    correctedColumns: validColumns,
    previewDigest: preview.digest,
    previewStatusFocused: preview.focusedStatus,
    definitionDigest,
    viewListed,
    inputAndActivationMode: "CDP keyboard events after Data view form render",
  };
}

async function exerciseDataFullKeyboardWorkflow(page, origin, nameComparison = "exact") {
  const assertFocusVisible = (...arguments_) => assertKeyboardFocusVisible(...arguments_, nameComparison);
  const replaceTextByKeyboard = (...arguments_) => replaceFocusedTextByKeyboard(...arguments_, nameComparison);
  const sql = "SELECT gpu_model, asking_usd_micros_per_hour, available_count FROM dataset ORDER BY gpu_model";
  const modelId = "full-keyboard-query-model";
  const viewId = "full-keyboard-cost-view";
  const validColumns = "gpu_model, asking_usd_micros_per_hour";
  await page.send("Emulation.setDeviceMetricsOverride", {
    width: 1440,
    height: 1000,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await page.send("DOM.enable");
  await page.send("Accessibility.enable");
  // The runner's earlier route/viewport audit is read-only. Reload before the
  // measured journey so this path begins at the real initial workspace state.
  await page.send("Page.navigate", { url: `${origin}/` });
  await waitForPageHeading(page);

  const dataNavigation = await assertFocusVisible(
    page,
    '.primary-nav [data-page="data"]',
    "Data navigation",
    "button",
    "Data",
  );
  await dispatchKeyboardEnter(page);
  await waitForBrowserValue(page, `(() => ({
    heading: document.querySelector("#workspace-page-title")?.textContent.trim() || "",
    active: document.querySelector('.primary-nav [data-page="data"]')?.getAttribute("aria-current") || "",
  }))()`, "keyboard-opened Data workspace", (value) => value.heading === "Data" && value.active === "page");

  const seedControl = await assertFocusVisible(
    page,
    '[data-action="seed-synthetic-data"]',
    "create synthetic snapshot",
    "button",
    "Create local synthetic snapshot",
  );
  await dispatchKeyboardEnter(page);
  await waitForBrowserValue(page,
    'Boolean(document.getElementById("data-dataset-id") && document.getElementById("data-query-form"))',
    "keyboard-seeded local Data catalog", (value) => value === true);

  const sqlControl = await assertFocusVisible(page, "#data-sql", "SQL query field", "textbox", "SQL");
  await replaceSingleLineTextareaByKeyboard(page, "#data-sql", sql, "SQL query");
  const querySubmit = await assertFocusVisible(
    page,
    '#data-query-form button[type="submit"]',
    "run bounded query",
    "button",
    "Run bounded query",
  );
  await assertKeyboardSubmitReady(page, "#data-query-form", '#data-query-form button[type="submit"]', "Data query");
  await dispatchKeyboardEnter(page);
  await waitForBrowserValue(page,
    'document.getElementById("data-query-status")?.textContent.trim() || ""',
    "keyboard query result",
    (value) => value.startsWith("Query complete:"));
  const result = await readDataWorkflowState(page);
  if (result.rowCount !== 3 || !/^[0-9a-f]{64}$/.test(result.snapshotDigest)
    || result.queryStatusRole !== "status" || result.queryStatusLive !== "polite") {
    throw new Error(`keyboard query result lost rows, provenance, or status semantics (${JSON.stringify(result)})`);
  }
  const resultRegion = await assertFocusVisible(
    page,
    '.data-table-scroll[aria-label="Query results table"]',
    "query results region",
    "region",
    "Query results table",
  );

  const modelNameField = await replaceTextByKeyboard(
    page,
    '#data-model-form input[name="model_id"]',
    modelId,
    "query model name",
    "textbox",
    "Model name",
  );
  const modelPreviewButton = await assertFocusVisible(
    page,
    '#data-model-form button[type="submit"]',
    "preview query model",
    "button",
    "Preview model",
  );
  await assertKeyboardSubmitReady(page, "#data-model-form", '#data-model-form button[type="submit"]', "query model preview");
  await dispatchKeyboardEnter(page);
  await waitForBrowserValue(page,
    'document.getElementById("data-model-status")?.textContent.trim() || ""',
    "keyboard model preview",
    (value) => value.startsWith("Preview ready."));
  const modelPreview = await page.evaluate(`(() => ({
    digest: document.querySelector(".data-preview-digest code")?.textContent.trim() || "",
    confirm: Boolean(document.querySelector('[data-action="confirm-data-model"]')),
    focused: document.activeElement?.id === "data-model-status",
    statusRole: document.getElementById("data-model-status")?.getAttribute("role") || "",
  }))()`);
  if (!/^[0-9a-f]{64}$/.test(modelPreview.digest) || !modelPreview.confirm
    || !modelPreview.focused || modelPreview.statusRole !== "status") {
    throw new Error(`keyboard model preview was not digest-bound and announced (${JSON.stringify(modelPreview)})`);
  }
  const modelConfirm = await assertFocusVisible(
    page,
    '[data-action="confirm-data-model"]',
    "confirm local model save",
    "button",
    "Confirm local save",
  );
  await dispatchKeyboardEnter(page);
  const modelStatus = await waitForBrowserValue(page,
    'document.getElementById("data-model-status")?.textContent.trim() || ""',
    "keyboard model save",
    (value) => value.startsWith("Saved model "));
  const modelDigest = modelStatus.match(/Definition digest ([0-9a-f]{64})\./)?.[1] || "";
  if (!modelDigest || !await page.evaluate(`Array.from(document.querySelectorAll(".data-definition-row strong"))
    .some((element) => element.textContent.trim() === ${JSON.stringify(modelId)})`)) {
    throw new Error("keyboard model confirmation did not save and list its definition digest");
  }

  const viewNameField = await replaceTextByKeyboard(
    page,
    '#data-view-form input[name="view_id"]',
    viewId,
    "view name",
    "textbox",
    "View name",
  );
  const presentation = await assertFocusVisible(
    page,
    '#data-view-form select[name="presentation"]',
    "view presentation selector",
    "combobox",
    "Presentation",
  );
  await dispatchKeyboardKey(page, "ArrowDown", "ArrowDown", 40);
  await dispatchKeyboardKey(page, "ArrowUp", "ArrowUp", 38);
  const presentationValue = await page.evaluate('document.querySelector(\'#data-view-form select[name="presentation"]\')?.value || ""');
  if (presentationValue !== "table") throw new Error(`keyboard presentation selection did not return to the frozen table mode (${presentationValue})`);

  const columnsSelector = '#data-view-form input[name="columns"]';
  const invalidColumnsField = await replaceTextByKeyboard(
    page,
    columnsSelector,
    "not a column",
    "invalid view columns",
    "textbox",
    "Columns",
  );
  const viewPreviewButton = await assertFocusVisible(
    page,
    '#data-view-form button[type="submit"]',
    "preview view",
    "button",
    "Preview view",
  );
  await assertKeyboardSubmitReady(page, "#data-view-form", '#data-view-form button[type="submit"]', "view preview");
  await dispatchKeyboardEnter(page);
  const failure = await waitForBrowserValue(page, `(() => {
    const status = document.getElementById("data-view-status");
    if (!status) return null;
    return {
      text: status.textContent.trim(),
      role: status.getAttribute("role"),
      live: status.getAttribute("aria-live"),
      atomic: status.getAttribute("aria-atomic"),
      focused: document.activeElement?.id === "data-view-status",
      viewId: document.querySelector('#data-view-form input[name="view_id"]')?.value || "",
      columns: document.querySelector(${JSON.stringify(columnsSelector)})?.value || "",
      confirm: Boolean(document.querySelector('[data-action="confirm-data-view"]')),
    };
  })()`, "keyboard view validation error", (value) => value?.text.startsWith("Preview failed:"));
  if (failure.role !== "status" || failure.live !== "polite" || failure.atomic !== "true"
    || !failure.focused || failure.viewId !== viewId || failure.columns !== "not a column" || failure.confirm) {
    throw new Error(`keyboard view rejection lost focus, values, or confirmation boundary (${JSON.stringify(failure)})`);
  }

  const correctedColumnsField = await replaceTextByKeyboard(
    page,
    columnsSelector,
    validColumns,
    "corrected view columns",
    "textbox",
    "Columns",
  );
  const correctedPreviewButton = await assertFocusVisible(
    page,
    '#data-view-form button[type="submit"]',
    "preview corrected view",
    "button",
    "Preview view",
  );
  await assertKeyboardSubmitReady(page, "#data-view-form", '#data-view-form button[type="submit"]', "corrected view preview");
  await dispatchKeyboardEnter(page);
  await waitForBrowserValue(page,
    'document.getElementById("data-view-status")?.textContent.trim() || ""',
    "keyboard corrected view preview",
    (value) => value.startsWith("Preview ready."));
  const viewPreview = await page.evaluate(`(() => ({
    digest: document.querySelector(".data-preview-digest code")?.textContent.trim() || "",
    confirm: Boolean(document.querySelector('[data-action="confirm-data-view"]')),
    focused: document.activeElement?.id === "data-view-status",
  }))()`);
  if (!/^[0-9a-f]{64}$/.test(viewPreview.digest) || !viewPreview.confirm || !viewPreview.focused) {
    throw new Error(`keyboard view preview was not digest-bound and focused (${JSON.stringify(viewPreview)})`);
  }
  const viewConfirm = await assertFocusVisible(
    page,
    '[data-action="confirm-data-view"]',
    "confirm local view save",
    "button",
    "Confirm view save",
  );
  await dispatchKeyboardEnter(page);
  const viewStatus = await waitForBrowserValue(page,
    'document.getElementById("data-view-status")?.textContent.trim() || ""',
    "keyboard view save",
    (value) => value.startsWith("Saved view "));
  const viewDigest = viewStatus.match(/Definition digest ([0-9a-f]{64})\./)?.[1] || "";
  const viewListed = await page.evaluate(`Array.from(document.querySelectorAll(".data-definition-row strong"))
    .some((element) => element.textContent.trim() === ${JSON.stringify(viewId)})`);
  if (!viewDigest || !viewListed) throw new Error("keyboard view confirmation did not save and list its definition digest");

  return {
    status: "passed",
    viewport: { width: 1440, height: 1000 },
    actionMode: "CDP keyboard only after initial page load",
    accessibleNameEvidence: "Chromium Accessibility.getPartialAXTree raw role/name pairs",
    accessibleNameComparison: nameComparison,
    dataNavigation,
    seedControl,
    sqlControl,
    querySubmit,
    resultRegion,
    modelNameField,
    modelPreviewButton,
    modelConfirm,
    viewNameField,
    presentation,
    invalidColumnsField,
    correctedColumnsField,
    viewPreviewButton,
    correctedPreviewButton,
    viewConfirm,
    resultRows: result.rowCount,
    datasetId: result.datasetId,
    snapshotDigest: result.snapshotDigest,
    modelId,
    modelPreviewDigest: modelPreview.digest,
    modelDefinitionDigest: modelDigest,
    invalidColumnsRetained: failure.columns === "not a column",
    errorStatusFocused: failure.focused,
    viewId,
    viewPreviewDigest: viewPreview.digest,
    viewDefinitionDigest: viewDigest,
    viewListed,
  };
}

async function exerciseACPKeyboardAXWorkflow(page, origin, { cancellation = false } = {}) {
  const nameComparison = "nfc-whitespace-ascii-case-insensitive";
  const profileName = cancellation
    ? "HyperAI ACP cancellation fixture"
    : "HyperAI ACP synthetic fixture";
  const fixtureExecutable = path.join(repositoryRoot, "target/debug/acp-fixture-peer");
  if (!fs.existsSync(fixtureExecutable)) {
    throw new Error("target/debug/acp-fixture-peer is missing; run the optional package script to build it first");
  }
  const replaceValueByKeyboard = async (selector, description, expectedName, value) => {
    const focus = await assertKeyboardFocusVisible(
      page,
      selector,
      description,
      "textbox",
      expectedName,
      nameComparison,
    );
    const selectAllModifier = process.platform === "darwin" ? 4 : 2;
    await dispatchKeyboardKey(page, "a", "KeyA", 65, selectAllModifier);
    await page.send("Input.insertText", { text: value });
    const retained = await page.evaluate(`(() => {
      const element = document.activeElement;
      return element?.matches(${JSON.stringify(selector)})
        && element.value.length === ${value.length};
    })()`);
    if (!retained) throw new Error(`keyboard text entry did not retain ${description}`);
    return { ...focus, enteredLength: value.length };
  };

  await page.send("Emulation.setDeviceMetricsOverride", {
    width: 1440,
    height: 1000,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await page.send("DOM.enable");
  await page.send("Accessibility.enable");
  // The preceding route and reduced-motion checks are read-only. Start the
  // measured workflow from a clean page and navigate with keyboard input.
  await page.send("Page.navigate", { url: `${origin}/` });
  await waitForPageHeading(page);

  const navigation = await assertKeyboardFocusVisible(
    page,
    '.primary-nav [data-page="access"]',
    "Agents & access navigation",
    "button",
    "Agents & access",
    nameComparison,
  );
  await dispatchKeyboardEnter(page);
  await waitForBrowserValue(
    page,
    `document.querySelector("#workspace-page-title")?.textContent.trim() || ""`,
    "keyboard-opened Agents & access workspace",
    (value) => value === "Agents & access",
  );

  const connectHeading = await assertComputedAccessibleNode(
    page,
    "#agent-workspace-title",
    "local ACP connect heading",
    "heading",
    "Connect a local agent",
    nameComparison,
  );
  const profileNameField = await replaceValueByKeyboard(
    '#agent-connect-form [name="name"]',
    "profile name",
    "Profile name",
    profileName,
  );
  const executableField = await replaceValueByKeyboard(
    '#agent-connect-form [name="executable"]',
    "executable path",
    "Absolute executable path",
    fixtureExecutable,
  );
  const workingDirectoryField = await replaceValueByKeyboard(
    '#agent-connect-form [name="working_directory"]',
    "working directory",
    "Working directory",
    os.tmpdir(),
  );
  const argumentsField = await assertKeyboardFocusVisible(
    page,
    '#agent-connect-form [name="args"]',
    "ACP arguments",
    "textbox",
    "Arguments one argument per line",
    nameComparison,
  );
  let fixtureArguments;
  if (cancellation) {
    await dispatchKeyboardText(page, "--wait-for-cancel");
    fixtureArguments = await page.evaluate(`(() => ({
      focused: document.activeElement?.matches('#agent-connect-form [name="args"]') || false,
      value: document.querySelector('#agent-connect-form [name="args"]')?.value || "",
    }))()`);
    if (!fixtureArguments.focused || fixtureArguments.value !== "--wait-for-cancel") {
      throw new Error("keyboard entry did not retain the fixed ACP cancellation fixture argument");
    }
  }
  const connectButton = await assertKeyboardFocusVisible(
    page,
    '#agent-connect-form button[type="submit"]',
    "review and connect",
    "button",
    "Review and connect",
    nameComparison,
  );
  await dispatchKeyboardEnter(page);
  const connected = await waitForBrowserValue(
    page,
    `(() => ({
      heading: document.querySelector("#agent-workspace-title")?.textContent.trim() || "",
      promptForm: Boolean(document.querySelector("#agent-prompt-form")),
      protocol: document.querySelector(".agent-session-panel .agent-panel-heading p:last-child")?.textContent || "",
      focused: document.activeElement?.id || "",
      toolCount: document.querySelectorAll(".agent-tool-list li").length,
    }))()`,
    "synthetic ACP session connection",
    (value) => value.heading === profileName
      && value.promptForm
      && value.protocol.includes("ACP v1")
      && value.focused === "agent-workspace-title"
      && value.toolCount > 0,
  );
  const connectedHeading = await assertComputedAccessibleNode(
    page,
    "#agent-workspace-title",
    "connected synthetic profile heading",
    "heading",
    profileName,
    nameComparison,
  );

  const promptField = await assertKeyboardFocusVisible(
    page,
    "#agent-prompt",
    "ACP prompt",
    "textbox",
    "Send a prompt to this local process",
    nameComparison,
  );
  await replaceFocusedTextByKeyboard(
    page,
    "#agent-prompt",
    "synthetic keyboard flow check",
    "synthetic ACP prompt",
    "textbox",
    "Send a prompt to this local process",
    nameComparison,
  );
  const sendPromptButton = await assertKeyboardFocusVisible(
    page,
    '#agent-prompt-form button[type="submit"]',
    "send ACP prompt",
    "button",
    "Send prompt",
    nameComparison,
  );
  await dispatchKeyboardEnter(page);
  let response;
  let pendingPrompt;
  let cancelButton;
  let cancellationResult;
  if (cancellation) {
    pendingPrompt = await waitForBrowserValue(
      page,
      `(() => ({
        submitText: document.querySelector('#agent-prompt-form button[type="submit"]')?.textContent.trim() || "",
        cancelDisabled: document.querySelector('[data-action="agent-cancel"]')?.hasAttribute("disabled") ?? true,
        focused: document.activeElement?.dataset.action || document.activeElement?.id || "",
      }))()`,
      "pending synthetic ACP prompt and keyboard cancel target",
      (value) => value.submitText === "Waiting for agent…"
        && !value.cancelDisabled
        && value.focused === "agent-cancel",
    );
    cancelButton = await assertComputedAccessibleNode(
      page,
      '[data-action="agent-cancel"]',
      "cancel active ACP request",
      "button",
      "Cancel active request",
      nameComparison,
    );
    await assertKeyboardFocusVisible(
      page,
      '[data-action="agent-cancel"]',
      "cancel active ACP request",
      "button",
      "Cancel active request",
      nameComparison,
    );
    await dispatchKeyboardEnter(page);
    cancellationResult = await waitForBrowserValue(
      page,
      `(() => ({
        status: document.querySelector(".agent-session-panel .agent-panel-heading p:last-child")?.textContent || "",
        text: document.querySelector(".agent-events ol li p")?.textContent.trim() || "",
        cancelDisabled: document.querySelector('[data-action="agent-cancel"]')?.hasAttribute("disabled") ?? true,
        focused: document.activeElement?.dataset.action || document.activeElement?.id || "",
      }))()`,
      "cancelled synthetic ACP prompt and focus restoration",
      (value) => value.status.includes("cancelled")
        && value.text === "Synthetic prompt is pending."
        && value.cancelDisabled
        && ["agent-prompt", "agent-workspace-title"].includes(value.focused),
    );
    response = {
      status: "cancelled",
      text: cancellationResult.text,
      focused: cancellationResult.focused,
    };
  } else {
    response = await waitForBrowserValue(
      page,
      `(() => ({
        text: document.querySelector(".agent-events ol li p")?.textContent.trim() || "",
        focused: document.activeElement?.id || "",
      }))()`,
      "synthetic ACP response and prompt focus restoration",
      (value) => value.text === "Synthetic ACP response." && value.focused === "agent-prompt",
    );
  }

  const disconnectButton = await assertKeyboardFocusVisible(
    page,
    '[data-action="agent-disconnect"]',
    "disconnect ACP session",
    "button",
    "Disconnect",
    nameComparison,
  );
  await dispatchKeyboardEnter(page);
  const disconnected = await waitForBrowserValue(
    page,
    `(() => ({
      connectForm: Boolean(document.querySelector("#agent-connect-form")),
      heading: document.querySelector("#agent-workspace-title")?.textContent.trim() || "",
      focused: document.activeElement?.id || "",
    }))()`,
    "synthetic ACP disconnect and focus restoration",
    (value) => value.connectForm
      && value.heading === "Connect a local agent"
      && value.focused === "agent-workspace-title",
  );
  const disconnectedHeading = await assertComputedAccessibleNode(
    page,
    "#agent-workspace-title",
    "disconnected connect heading",
    "heading",
    "Connect a local agent",
    nameComparison,
  );

  return {
    status: "passed",
    viewport: { width: 1440, height: 1000 },
    actionMode: "Tab/Enter keyboard navigation and activation; CDP text insertion into keyboard-focused fields",
    accessibleNameEvidence: "Chromium Accessibility.getPartialAXTree raw role/name pairs",
    accessibleNameComparison: nameComparison,
    navigation,
    connectHeading,
    profileNameField,
    executableField,
    workingDirectoryField,
    argumentsField,
    ...(fixtureArguments ? { fixtureArguments } : {}),
    connectButton,
    connected,
    connectedHeading,
    promptField,
    sendPromptButton,
    response,
    ...(pendingPrompt ? { pendingPrompt } : {}),
    ...(cancelButton ? { cancelButton } : {}),
    ...(cancellationResult ? { cancellationResult } : {}),
    disconnectButton,
    disconnected,
    disconnectedHeading,
    toolCallsInvoked: 0,
    expectedAgentPostPaths: [
      "/api/v1/agent/connect",
      "/api/v1/agent/prompt",
      ...(cancellation ? ["/api/v1/agent/cancel"] : []),
      "/api/v1/agent/disconnect",
    ],
  };
}

async function exerciseWorkloadTraceSemanticWorkflow(page, chromeVersion) {
  const navigation = await assertKeyboardFocusVisible(
    page,
    '.primary-nav [data-page="deals"]',
    "Deals navigation",
    "button",
    "Deals",
  );
  await dispatchKeyboardEnter(page);
  const dealsHeading = await waitForBrowserValue(
    page,
    `(() => ({
      heading: document.querySelector("#workspace-page-title")?.textContent.trim() || "",
      route: document.querySelector('[aria-current="page"][data-page]')?.dataset.page || "",
      focused: document.activeElement?.id || "",
      focusVisible: document.activeElement?.matches(":focus-visible") || false,
    }))()`,
    "keyboard navigation to the Deals workspace",
    (value) => value.heading === "Deals" && value.route === "deals" && value.focused === "workspace-page-title",
  );
  if (!dealsHeading.focusVisible) throw new Error("Deals navigation did not leave visible focus on the workspace heading");
  const workspaceHeading = await assertComputedAccessibleNode(
    page,
    "#workspace-page-title",
    "Deals workspace heading",
    "heading",
    "Deals",
  );
  const traceReady = await waitForBrowserValue(
    page,
    `(() => ({
      heading: document.querySelector(".workload-trace-panel h2")?.textContent.trim() || "",
      termCount: document.querySelectorAll(".workload-trace-panel dl.trace-summary dt").length,
      valueCount: document.querySelectorAll(".workload-trace-panel dl.trace-summary dd").length,
      digest: document.querySelector("#workload-trace-digest-description")?.textContent.trim() || "",
      visualDigest: document.querySelector(".workload-trace-panel .trace-summary dd code[aria-hidden='true']")?.textContent.trim() || "",
      digestDescriptionTarget: document.querySelector("#workload-trace-digest-value")?.getAttribute("aria-describedby") || "",
      sourceDigest: (() => {
        const reference = Array.from(document.querySelectorAll(".workload-trace-panel .trace-reference"))
          .find((element) => {
            const description = document.getElementById(element.getAttribute("aria-describedby") || "")?.textContent.trim() || "";
            return description.startsWith("Source-provided digest: ");
          });
        if (!reference) return null;
        const descriptionId = reference.getAttribute("aria-describedby");
        return {
          selector: ".workload-trace-panel .trace-reference[aria-describedby=" + descriptionId + "]",
          fullDescription: document.getElementById(descriptionId)?.textContent.trim() || "",
          visualDigest: reference.querySelector('code[aria-hidden="true"]')?.textContent.trim() || "",
        };
      })(),
    }))()`,
    "populated successful Workload Trace",
    (value) => value.heading === "One workload, source-owned records" && value.termCount === 3
      && value.valueCount === 3 && value.digest.length > 0 && value.visualDigest.length > 0,
  );

  const region = await assertComputedAccessibleNode(
    page,
    ".workload-trace-panel",
    "successful Workload Trace panel",
    "region",
    "One workload, source-owned records",
  );
  const refreshButton = await assertComputedAccessibleNode(
    page,
    '.workload-trace-panel [data-action="load-workload-trace"]',
    "refresh successful Workload Trace",
    "button",
    "Refresh trace",
  );
  const definitionList = await readComputedAccessibleNode(page, ".workload-trace-panel dl.trace-summary");
  const workloadTerm = await readComputedAccessibleNode(page, ".workload-trace-panel dl.trace-summary dt:first-of-type");
  const workloadValue = await readComputedAccessibleNode(page, ".workload-trace-panel dl.trace-summary dd:first-of-type");
  const descriptionRoleExpectations = [
    ["definition list", definitionList, ["DescriptionList"]],
    ["definition term", workloadTerm, ["DescriptionListTerm", "Term", "term"]],
    ["definition value", workloadValue, ["DescriptionListDetail", "Definition", "definition"]],
  ];
  for (const [description, accessible, expectedRoles] of descriptionRoleExpectations) {
    if (!accessible.present || accessible.ignored || !expectedRoles.includes(accessible.role)) {
      throw new Error(`Chromium did not expose ${description} semantics (${JSON.stringify({ expectedRoles, actual: accessible })})`);
    }
  }

  if (traceReady.visualDigest === traceReady.digest
    || traceReady.digestDescriptionTarget !== "workload-trace-digest-description") {
    throw new Error(`Workload Trace digest full/visual DOM representations differ from the contract (${JSON.stringify({
      digestLength: traceReady.digest.length,
      visualDigestDiffers: traceReady.visualDigest !== traceReady.digest,
      digestDescriptionTarget: traceReady.digestDescriptionTarget,
    })})`);
  }
  const fullTree = await page.send("Accessibility.getFullAXTree");
  const digestValue = await readComputedAccessibleNode(page, "#workload-trace-digest-value");
  if (!traceReady.sourceDigest) throw new Error("the local successful trace contains no source digest to qualify");
  const sourceDigestValue = await readComputedAccessibleNode(page, traceReady.sourceDigest.selector);
  const axValues = (fullTree.nodes || []).flatMap((node) => [
    String(node.name?.value || ""),
    String(node.description?.value || ""),
  ]);
  if (digestValue.description.toLowerCase() !== traceReady.digest.toLowerCase()
    || sourceDigestValue.description.toLowerCase() !== traceReady.sourceDigest.fullDescription.toLowerCase()
    || axValues.some((value) => value.includes(traceReady.visualDigest)
      || value.includes(traceReady.sourceDigest.visualDigest))) {
    throw new Error(`Chromium's accessibility tree did not expose the full digest description while excluding its visual abbreviation (${JSON.stringify({
      descriptionLength: digestValue.description.length,
      descriptionMatchesDigestCaseInsensitively: digestValue.description.toLowerCase() === traceReady.digest.toLowerCase(),
      sourceDescriptionLength: sourceDigestValue.description.length,
      sourceDescriptionMatches: sourceDigestValue.description.toLowerCase() === traceReady.sourceDigest.fullDescription.toLowerCase(),
      descriptionPrefix: digestValue.description.slice(0, 12),
      descriptionSuffix: digestValue.description.slice(-8),
      digestPrefix: traceReady.digest.slice(0, 12),
      digestSuffix: traceReady.digest.slice(-8),
      visualDigestExposed: axValues.some((value) => value.includes(traceReady.visualDigest)
        || value.includes(traceReady.sourceDigest.visualDigest)),
    })})`);
  }

  const activatePreview = async ({ selector, name, heading: expectedHeading, expectedRegionName }) => {
    const button = await assertKeyboardFocusVisible(
      page,
      selector,
      name,
      "button",
      name,
    );
    await dispatchKeyboardEnter(page);
    const state = await waitForBrowserValue(
      page,
      `(() => ({
        heading: document.querySelector(".workload-trace-panel h2")?.textContent.trim() || "",
        focused: document.activeElement?.id || "",
        focusVisible: document.activeElement?.matches(":focus-visible") || false,
      }))()`,
      `${name} activation and focus restoration`,
      (value) => value.heading === expectedHeading && value.focused === "workload-trace-heading",
    );
    if (!state.focusVisible) throw new Error(`${name} did not restore visible keyboard focus to its updated heading`);
    const updatedRegion = await assertComputedAccessibleNode(
      page,
      ".workload-trace-panel",
      `${name} preview panel`,
      "region",
      expectedRegionName,
    );
    return { button, heading: state.heading, focusRestored: state.focused, region: updatedRegion };
  };

  const sellerFailure = await activatePreview({
    selector: '.workload-trace-panel [data-action="preview-seller-failure"]',
    name: "Preview seller-reported failure",
    heading: "Seller-reported failure preview",
    expectedRegionName: "Seller-reported failure preview",
  });
  const inconclusive = await activatePreview({
    selector: '.workload-trace-panel [data-action="preview-inconclusive-outcome"]',
    name: "Preview inconclusive mock outcome",
    heading: "Inconclusive verifier preview",
    expectedRegionName: "Inconclusive verifier preview",
  });
  const successfulReturn = await activatePreview({
    selector: '.workload-trace-panel [data-action="load-workload-trace"]',
    name: "Return to successful trace",
    heading: "One workload, source-owned records",
    expectedRegionName: "One workload, source-owned records",
  });

  return {
    status: "passed",
    viewport: { width: 1440, height: 1000 },
    chromeVersion,
    actionMode: "Tab/Enter keyboard navigation and activation",
    accessibleNameEvidence: "Chromium Accessibility.getPartialAXTree and getFullAXTree",
    dealsNavigation: navigation,
    dealsHeading: { ...dealsHeading, accessibleRole: workspaceHeading.role, accessibleName: workspaceHeading.name },
    refreshButton,
    successfulTrace: {
      region: region.name,
      descriptionListRoles: {
        list: definitionList.role,
        term: workloadTerm.role,
        value: workloadValue.role,
      },
      fullDigestAccessibleDescriptionLength: digestValue.description.length,
      sourceDigestAccessibleDescriptionLength: sourceDigestValue.description.length,
      visualAbbreviationHiddenFromAccessibilityTree: true,
      sourceAbbreviationHiddenFromAccessibilityTree: true,
      digestLength: traceReady.digest.length,
      visualDigest: traceReady.visualDigest,
    },
    sellerFailure,
    inconclusive,
    successfulReturn,
  };
}

async function exerciseMarketSupplyComparisonWorkflow(page, chromeVersion, includeQuotePreview = false, includeBuyerLifecycle = false, injectStaleComparison = false) {
  await page.send("Emulation.setDeviceMetricsOverride", {
    width: 1440,
    height: 1000,
    deviceScaleFactor: 1,
    mobile: false,
  });
  const navigation = await assertKeyboardFocusVisible(
    page,
    '.primary-nav [data-page="market"]',
    "Market navigation",
    "button",
    "Market",
  );
  await dispatchKeyboardEnter(page);
  const marketHeading = await waitForBrowserValue(
    page,
    '(() => ({ heading: document.querySelector("#workspace-page-title")?.textContent.trim() || "", route: document.querySelector(\'[aria-current="page"][data-page]\')?.dataset.page || "", focused: document.activeElement?.id || "", focusVisible: document.activeElement?.matches(":focus-visible") || false }))()',
    "keyboard navigation to the Market workspace",
    (value) => value.heading === "Market" && value.route === "market" && value.focused === "workspace-page-title",
  );
  if (!marketHeading.focusVisible) throw new Error("Market navigation did not leave visible focus on the workspace heading");

  const comparison = await waitForBrowserValue(
    page,
    '(() => { const panel = document.querySelector(".supply-panel"); const table = panel?.querySelector(".supply-table table"); const rows = Array.from(panel?.querySelectorAll(".supply-table tbody tr") || []); const selected = panel?.querySelector(".supply-selected"); const scroll = panel?.querySelector(".supply-table .table-scroll"); return { heading: panel?.querySelector("#supply-heading")?.textContent.trim() || "", offerCount: rows.length, offerIds: rows.map((row) => row.querySelector("td strong")?.textContent.trim() || ""), selectedText: selected?.textContent.trim() || "", selectedOfferId: selected?.closest("tr")?.querySelector("td strong")?.textContent.trim() || "", comparisonDigest: panel?.querySelector(".supply-comparison-digest")?.textContent.trim().replace(/^COMPARISON\\s+/, "") || "", quoteAction: panel?.querySelector("[data-action=preview-supply-quote]")?.getAttribute("aria-label") || "", observationCount: panel?.querySelectorAll(".supply-observation").length || 0, advisoryNotOfferCount: panel?.querySelectorAll(".supply-observation .tag-caution").length || 0, bodyText: panel?.innerText || "", tableCaption: table?.querySelector("caption")?.textContent.trim() || "", tableScroll: scroll ? { clientWidth: scroll.clientWidth, scrollWidth: scroll.scrollWidth, overflowX: getComputedStyle(scroll).overflowX } : null, captureHeading: document.querySelector(".market-capture-section h2")?.textContent.trim() || "" }; })()',
    "the fixed Market supply comparison",
    (value) => value.heading === "Compare the request with signed offers" && value.offerCount === 2 && value.selectedText.length > 0,
  );
  if (!comparison.offerIds.includes("offer-gpu-b")
    || comparison.selectedText !== "Selected by asking price"
    || comparison.selectedOfferId !== "offer-gpu-b"
    || !/^[0-9a-f]{64}$/.test(comparison.comparisonDigest)
    || comparison.quoteAction !== "Preview quote for offer-gpu-b"
    || comparison.observationCount < 1
    || comparison.advisoryNotOfferCount < 1
    || comparison.tableCaption !== "Offer eligibility and seller asking terms for the fixed buyer request"
    || comparison.captureHeading !== "Simple Compute Market captures") {
    throw new Error("the fixed Market supply comparison did not retain its offer, advisory, or capture boundaries (" + JSON.stringify(comparison) + ")");
  }
  for (const phrase of [
    "BUYER CONSTRAINTS",
    "SIGNED CATALOG",
    "Seller asking",
    "Request total",
    "Freshness: not_evaluated",
    "Not an offer",
    "no live supply",
    "fixed local synthetic signed catalog only",
    "no live availability",
    "Read-only in-memory synthetic projection",
  ]) {
    if (!comparison.bodyText.toLowerCase().includes(phrase.toLowerCase())) {
      throw new Error("the buyer comparison omitted required visible evidence: " + phrase);
    }
  }
  const region = await assertComputedAccessibleNode(
    page,
    ".supply-panel",
    "buyer supply comparison",
    "region",
    "Compare the request with signed offers",
  );
  const table = await assertComputedAccessibleNode(
    page,
    ".supply-table table",
    "signed offers table",
    "table",
    "Offer eligibility and seller asking terms for the fixed buyer request",
  );

  await page.send("Emulation.setDeviceMetricsOverride", {
    width: 390,
    height: 844,
    deviceScaleFactor: 1,
    mobile: true,
  });
  await delay(80);
  const mobileLayout = await page.evaluate('(() => { const scroll = document.querySelector(".supply-table .table-scroll"); return { viewportWidth: document.documentElement.clientWidth, documentScrollWidth: document.documentElement.scrollWidth, tableScrollWidth: scroll?.scrollWidth || 0, tableClientWidth: scroll?.clientWidth || 0, overflowX: scroll ? getComputedStyle(scroll).overflowX : "missing" }; })()');
  if (mobileLayout.documentScrollWidth > mobileLayout.viewportWidth
    || mobileLayout.overflowX !== "auto"
    || mobileLayout.tableScrollWidth <= mobileLayout.tableClientWidth) {
    throw new Error("the 390px Market comparison did not keep horizontal scrolling inside the offers table (" + JSON.stringify(mobileLayout) + ")");
  }
  let quotePreview;
  let quoteActionFocus;
  let buyerLifecycle;
  if (includeQuotePreview) {
    await page.send("Emulation.setDeviceMetricsOverride", {
      width: 1440,
      height: 1000,
      deviceScaleFactor: 1,
      mobile: false,
    });
    quoteActionFocus = await assertKeyboardFocusVisible(
      page,
      '[data-action="preview-supply-quote"]',
      "selected-offer quote preview",
      "button",
      "Preview quote for offer-gpu-b",
    );
    if (injectStaleComparison) {
      await page.evaluate(`(() => {
        const originalFetch = window.fetch.bind(window);
        window.__hyperaiStaleQuoteRequest = null;
        window.fetch = async (input, init = {}) => {
          const url = typeof input === "string" ? input : input.url;
          const method = String(init.method || (input instanceof Request ? input.method : "GET")).toUpperCase();
          if (new URL(url, window.location.href).pathname === "/api/v1/market/supply-quote-preview" && method === "POST") {
            const body = JSON.parse(init.body || "{}");
            const staleDigest = "0".repeat(64);
            window.__hyperaiStaleQuoteRequest = {
              displayedDigest: document.querySelector(".supply-comparison-digest")?.textContent.trim().replace(/^COMPARISON\\s+/, "") || "",
              clientDigest: body.comparison_digest || "",
              staleDigest,
            };
            const response = await originalFetch(input, { ...init, body: JSON.stringify({ ...body, comparison_digest: staleDigest }) });
            const envelope = await response.clone().json().catch(() => null);
            const validEnvelope = envelope !== null
              && typeof envelope === "object"
              && !Array.isArray(envelope)
              && envelope.schema_version === "hyperai.interface.error.v1"
              && envelope.status === "error"
              && typeof envelope.code === "string"
              && /^[A-Z0-9_-]{1,80}$/.test(envelope.code);
            window.__hyperaiStaleQuoteRequest = {
              ...window.__hyperaiStaleQuoteRequest,
              responseStatus: response.status,
              responseCode: validEnvelope ? envelope.code : "UNKNOWN_ERROR",
            };
            return response;
          }
          return originalFetch(input, init);
        };
        return true;
      })()`);
    }
    await dispatchKeyboardEnter(page);
    if (injectStaleComparison) {
      const staleFailure = await waitForBrowserValue(
        page,
        '(() => { const root = document.querySelector("#page-root"); const status = root?.querySelector(".supply-error[role=status][aria-live=polite]"); const style = status ? getComputedStyle(status) : null; return { route: document.querySelector("[aria-current=page][data-page]")?.dataset.page || "", heading: document.querySelector("#workspace-page-title")?.textContent.trim() || "", displayedDigest: document.querySelector(".supply-comparison-digest")?.textContent.trim().replace(/^COMPARISON\\s+/, "") || "", errorText: status?.textContent.trim() || "", errorRole: status?.getAttribute("role") || "", errorLive: status?.getAttribute("aria-live") || "", errorTabIndex: status?.getAttribute("tabindex") || "", errorFocused: Boolean(status && document.activeElement === status), errorFocusVisible: Boolean(status?.matches(":focus-visible")), errorOutlineStyle: style?.outlineStyle || "", errorOutlineWidth: style?.outlineWidth || "", quoteDetails: Boolean(root?.querySelector(".quote-digest-panel")), approval: Boolean(root?.querySelector(".quote-approval-result")), completion: Boolean(root?.querySelector(".quote-completion-link")), interception: window.__hyperaiStaleQuoteRequest }; })()',
        "the stale-comparison quote-preview failure",
        (value) => value.route === "market"
          && value.interception?.responseStatus === 409
          && value.interception?.responseCode === "MARKET_SUPPLY_COMPARISON_CHANGED"
          && value.errorText.length > 0,
      );
      if (!/^[0-9a-f]{64}$/.test(staleFailure.displayedDigest)
        || staleFailure.displayedDigest !== comparison.comparisonDigest
        || staleFailure.interception.displayedDigest !== comparison.comparisonDigest
        || staleFailure.interception.clientDigest !== comparison.comparisonDigest
        || staleFailure.interception.staleDigest !== "0".repeat(64)
        || staleFailure.errorRole !== "status"
        || staleFailure.errorLive !== "polite"
        || staleFailure.errorTabIndex !== "-1"
        || !staleFailure.errorFocused
        || !staleFailure.errorFocusVisible
        || staleFailure.errorOutlineStyle === "none"
        || Number.parseFloat(staleFailure.errorOutlineWidth) <= 0
        || staleFailure.interception.responseStatus !== 409
        || staleFailure.interception.responseCode !== "MARKET_SUPPLY_COMPARISON_CHANGED"
        || staleFailure.errorText !== "This supply comparison is out of date. Reload this page to get a fresh comparison before requesting a quote."
        || staleFailure.quoteDetails || staleFailure.approval || staleFailure.completion) {
        throw new Error("the stale comparison failure did not remain accessible and fail closed before approval (" + JSON.stringify(staleFailure) + ")");
      }
      staleFailure.accessibleStatus = await assertComputedAccessibleNode(
        page,
        '.supply-error[role="status"][aria-live="polite"]',
        "stale quote-preview recovery status",
        "status",
        "This supply comparison is out of date. Reload this page to get a fresh comparison before requesting a quote.",
      );
      return {
        navigation,
        marketHeading,
        comparison,
        quoteActionFocus,
        staleFailure,
        chromeVersion,
      };
    }
    quotePreview = await waitForBrowserValue(
      page,
      '(() => { const root = document.querySelector("#page-root"); const rows = Array.from(root?.querySelectorAll(".quote-digest-panel .detail-list > div") || []); const valueFor = (label) => rows.find((row) => row.querySelector("dt")?.textContent.trim() === label)?.querySelector("dd")?.textContent.trim() || ""; const button = root?.querySelector("[data-action=approve-quote]"); const quoteDigests = Array.from(root?.querySelectorAll(".quote-digest-panel code") || []).map((node) => node.textContent.trim()); return { heading: root?.querySelector("#workspace-page-title")?.textContent.trim() || "", selectedOffer: root?.querySelector(".deal-banner h2")?.textContent.trim() || "", comparisonDigest: valueFor("Comparison digest"), catalogDigest: valueFor("Catalog digest"), signedOfferDigest: valueFor("Signed offer digest"), buyerRequestDigest: valueFor("Buyer request digest"), bindingDigest: valueFor("Supply-to-quote digest"), quotePreviewDigest: quoteDigests.find((value) => /^[0-9a-f]{64}$/.test(value)) || "", quotedAmount: root?.querySelector(".deal-amount strong")?.textContent.trim() || "", bodyText: root?.innerText || "", approvalButton: button?.textContent.trim() || "", approvalResult: Boolean(root?.querySelector(".quote-approval-result")), completion: Boolean(root?.querySelector("[data-action=complete-quote]")) }; })()',
      "the selected-offer quote preview review",
      (value) => value.heading === "Review seller quote" && value.selectedOffer === "offer-gpu-b" && value.bindingDigest.length === 64,
    );
    if (quotePreview.comparisonDigest !== comparison.comparisonDigest
      || !/^[0-9a-f]{64}$/.test(quotePreview.catalogDigest)
      || !/^[0-9a-f]{64}$/.test(quotePreview.signedOfferDigest)
      || !/^[0-9a-f]{64}$/.test(quotePreview.buyerRequestDigest)
      || !/^[0-9a-f]{64}$/.test(quotePreview.quotePreviewDigest)
      || !quotePreview.quotedAmount
      || !quotePreview.bodyText.toLowerCase().includes("no seller, provider, network, storage, approval, fulfillment, or funds effect")
      || quotePreview.approvalButton !== "Approve synthetic quote"
      || quotePreview.approvalResult
      || quotePreview.completion) {
      throw new Error("the quote review did not show the selected-offer binding and separate approval boundary (" + JSON.stringify(quotePreview) + ")");
    }
    if (includeBuyerLifecycle) {
      const approvalActionFocus = await assertKeyboardFocusVisible(
        page,
        '[data-action="approve-quote"]',
        "synthetic quote approval",
        "button",
        "Approve synthetic quote",
      );
      await dispatchKeyboardEnter(page);
      const approval = await waitForBrowserValue(
        page,
        '(() => { const root = document.querySelector("#page-root"); const rows = Array.from(root?.querySelectorAll(".quote-approval-result .detail-list > div") || []); const valueFor = (items, label) => items.find((row) => row.querySelector("dt")?.textContent.trim() === label)?.querySelector("dd")?.textContent.trim() || ""; const historyRows = Array.from(root?.querySelectorAll(".market-history-row") || []); const historyDeal = Array.from(historyRows[0]?.querySelectorAll(".detail-list > div") || []).find((row) => row.querySelector("dt")?.textContent.trim() === "Deal digest"); return { heading: root?.querySelector("#quote-approval-heading")?.textContent.trim() || "", focused: document.activeElement?.id || "", previewDigest: valueFor(rows, "Bound preview"), dealDigest: valueFor(rows, "Deal digest"), completionAvailable: Boolean(root?.querySelector("[data-action=complete-quote]")), bodyText: root?.querySelector(".quote-approval-result")?.innerText || "", historyStatus: historyRows[0]?.querySelector(".tag")?.textContent.trim() || "", historyDealDigest: historyDeal?.querySelector("dd code")?.getAttribute("title") || "" }; })()',
        "the explicit synthetic buyer approval",
        (value) => value.heading === "Quote bound to a local deal" && value.previewDigest.length === 64 && value.dealDigest.length === 64 && value.completionAvailable && value.historyStatus === "APPROVED LOCALLY",
      );
      if (approval.focused !== "quote-approval-heading"
        || approval.previewDigest !== quotePreview.quotePreviewDigest
        || approval.historyDealDigest !== approval.dealDigest
        || !approval.bodyText.includes("This is not purchase authorization")
        || !approval.bodyText.toLowerCase().includes("no real buyer identity, seller call, fulfillment, verifier, settlement, or funds movement")) {
        throw new Error("the synthetic approval did not preserve digest binding, visible boundary text, and focus restoration (" + JSON.stringify(approval) + ")");
      }

      const completionActionFocus = await assertKeyboardFocusVisible(
        page,
        '[data-action="complete-quote"]',
        "mock fulfillment",
        "button",
        "Continue to mock fulfillment",
      );
      await dispatchKeyboardEnter(page);
      const completion = await waitForBrowserValue(
        page,
        '(() => { const root = document.querySelector("#page-root"); const rows = Array.from(root?.querySelectorAll(".quote-completion-link .detail-list > div") || []); const valueFor = (items, label) => items.find((row) => row.querySelector("dt")?.textContent.trim() === label)?.querySelector("dd")?.textContent.trim() || ""; const panel = root?.querySelector(".quote-completion-link"); const historyRows = Array.from(root?.querySelectorAll(".market-history-row") || []); const historyDetails = Array.from(historyRows[0]?.querySelectorAll(".detail-list > div") || []); const digestFor = (label) => historyDetails.find((row) => row.querySelector("dt")?.textContent.trim() === label)?.querySelector("dd code")?.getAttribute("title") || ""; return { heading: root?.querySelector("#quote-completion-heading")?.textContent.trim() || "", focused: document.activeElement?.id || "", previewDigest: valueFor(rows, "Preview digest"), dealDigest: valueFor(rows, "Approved deal"), receiptDigest: valueFor(rows, "Receipt"), bodyText: panel?.innerText || "", pageText: root?.innerText || "", historyStatus: historyRows[0]?.querySelector(".tag")?.textContent.trim() || "", historyDealDigest: digestFor("Deal digest"), historyReceiptDigest: digestFor("Mock receipt") }; })()',
        "the synthetic mock fulfillment and receipt",
        (value) => value.heading === "Receipt continues the quote you reviewed" && value.previewDigest.length === 64 && value.dealDigest.length === 64 && value.receiptDigest.length === 64 && value.historyStatus === "MOCK COMPLETED",
      );
      if (completion.focused !== "quote-completion-heading"
        || completion.previewDigest !== quotePreview.quotePreviewDigest
        || completion.dealDigest !== approval.dealDigest
        || completion.historyDealDigest !== completion.dealDigest
        || completion.historyReceiptDigest !== completion.receiptDigest
        || !completion.bodyText.toLowerCase().includes("no funds move")
        || !completion.bodyText.toLowerCase().includes("deterministic fixture statement")
        || !completion.bodyText.toLowerCase().includes("not independent correctness or payment evidence")
        || !completion.pageText.toLowerCase().includes("unsigned local mock ledger · no funds moved")) {
        throw new Error("the synthetic receipt did not preserve preview/deal binding, claim limits, and focus restoration (" + JSON.stringify(completion) + ")");
      }
      buyerLifecycle = {
        approval: {
          actionFocus: approvalActionFocus,
          restoredHeadingFocus: approval.focused,
          previewDigest: approval.previewDigest,
          dealDigest: approval.dealDigest,
          explicitNonPurchaseBoundary: true,
        },
        completion: {
          actionFocus: completionActionFocus,
          restoredHeadingFocus: completion.focused,
          previewDigest: completion.previewDigest,
          dealDigest: completion.dealDigest,
          receiptDigest: completion.receiptDigest,
          mockOnlyBoundary: true,
        },
      };
    }
  }
  return {
    status: "passed",
    chromeVersion,
    keyboardNavigation: navigation,
    marketHeading: { ...marketHeading, accessibleRole: "heading", accessibleName: "Market" },
    comparison: {
      region: { role: region.role, name: region.name },
      table: { role: table.role, name: table.name, caption: comparison.tableCaption },
      assessedOffers: comparison.offerCount,
      offerIds: comparison.offerIds,
      selectedOffer: comparison.selectedOfferId,
      comparisonDigest: comparison.comparisonDigest,
      selectionLabel: comparison.selectedText,
      advisoryObservations: comparison.observationCount,
      advisoryNotOfferLabels: comparison.advisoryNotOfferCount,
      sellerAskingAndRequestTotalVisible: true,
      freshness: "not_evaluated",
      separateCaptureExplorer: comparison.captureHeading,
      visibleClaimAndBoundaryText: true,
    },
    quotePreview: quotePreview ? {
      keyboardAction: quoteActionFocus,
      selectedOffer: quotePreview.selectedOffer,
      comparisonDigestBound: quotePreview.comparisonDigest,
      catalogDigest: quotePreview.catalogDigest,
      signedOfferDigest: quotePreview.signedOfferDigest,
      buyerRequestDigest: quotePreview.buyerRequestDigest,
      supplyToQuoteDigest: quotePreview.bindingDigest,
      quotePreviewDigest: quotePreview.quotePreviewDigest,
      quotedAmount: quotePreview.quotedAmount,
      approvalVisibleAsSeparateAction: quotePreview.approvalButton,
      approvalCreated: quotePreview.approvalResult,
      completionCreated: quotePreview.completion,
    } : undefined,
    buyerLifecycle,
    mobileLayout: {
      viewportWidth: mobileLayout.viewportWidth,
      documentScrollWidth: mobileLayout.documentScrollWidth,
      tableScrollWidth: mobileLayout.tableScrollWidth,
      tableClientWidth: mobileLayout.tableClientWidth,
      overflowX: mobileLayout.overflowX,
    },
  };
}

async function exerciseTaskNavigationKeyboardWorkflow(page, origin, chromeVersion, networkRequests) {
  const groups = [
    { label: "WORK", routes: [["overview", "Overview", "Compute, with context."], ["market", "Market"], ["deals", "Deals"]] },
    { label: "RESEARCH", routes: [["data", "Data"], ["eval", "Eval"], ["trade", "Trade"]] },
    { label: "OPERATE", routes: [["inference", "Inference"], ["fleet", "Fleet"], ["evidence", "Evidence"], ["access", "Agents & access"], ["activity", "Activity"]] },
  ];
  const expectedGroups = groups.map(({ label, routes: groupedRoutes }) => ({
    label,
    routes: groupedRoutes.map(([route]) => route),
  }));
  const observedGroups = await page.evaluate(`(() => {
    const groups = [];
    let current;
    for (const child of document.querySelector(".primary-nav")?.children || []) {
      if (child.matches(".nav-label")) {
        current = { label: child.textContent.trim(), routes: [] };
        groups.push(current);
      } else if (child.matches(".nav-item[data-page]") && current) {
        current.routes.push(child.dataset.page);
      }
    }
    return groups;
  })()`);
  if (JSON.stringify(observedGroups) !== JSON.stringify(expectedGroups)) {
    throw new Error(`the Work / Research / Operate navigation groups changed (${JSON.stringify({ expectedGroups, observedGroups })})`);
  }

  const routeResults = [];
  const groupedRoutes = groups.flatMap(({ routes: groupRoutes }) => groupRoutes);
  for (let index = 0; index < groupedRoutes.length; index += 1) {
    if (index > 0) {
      await page.send("Page.reload", { ignoreCache: true });
      await delay(350);
      await waitForPageHeading(page);
    }
    const [route, expectedName, expectedHeading = expectedName] = groupedRoutes[index];
    const navigation = await assertKeyboardFocusVisible(
      page,
      `.primary-nav [data-page="${route}"]`,
      `${expectedName} navigation`,
      "button",
      expectedName,
    );
    await dispatchKeyboardEnter(page);
    const rendered = await waitForBrowserValue(
      page,
      `(() => {
        const active = document.querySelectorAll('.primary-nav [aria-current="page"][data-page]');
        const heading = document.querySelectorAll("#page-root h1");
        return {
          route: active[0]?.dataset.page || "",
          activeCount: active.length,
          heading: heading[0]?.textContent.trim() || "",
          headingCount: heading.length,
          focus: document.activeElement?.id || "",
          focusVisible: document.activeElement?.matches(":focus-visible") || false,
        };
      })()`,
      `keyboard navigation to ${expectedName}`,
      (value) => value.route === route && value.heading === expectedHeading && value.focus === "workspace-page-title",
    );
    if (rendered.activeCount !== 1 || rendered.headingCount !== 1 || !rendered.focusVisible) {
      throw new Error(`${expectedName} did not retain one active route, one heading, and visible heading focus (${JSON.stringify(rendered)})`);
    }
    const heading = await assertComputedAccessibleNode(
      page,
      "#workspace-page-title",
      `${expectedHeading} workspace heading`,
      "heading",
      expectedHeading,
    );
    routeResults.push({
      route,
      accessibleName: navigation.accessibleName,
      tabFocusVisible: navigation.focusVisible,
      heading: heading.name,
      headingFocusVisible: rendered.focusVisible,
      activeCount: rendered.activeCount,
      headingCount: rendered.headingCount,
    });
  }

  await page.send("Emulation.setDeviceMetricsOverride", {
    width: 390,
    height: 844,
    deviceScaleFactor: 1,
    mobile: true,
  });
  await page.send("Page.reload", { ignoreCache: true });
  await delay(350);
  await waitForPageHeading(page);
  const initialMobileFocus = await page.evaluate("document.activeElement === document.body");
  if (!initialMobileFocus) throw new Error("the mobile keyboard workflow did not begin at the document body");

  await dispatchKeyboardKey(page, "Tab", "Tab", 9);
  const skipLinkFocus = await page.evaluate(`(() => ({
    matches: Boolean(document.activeElement?.matches(".skip-link")),
    focusVisible: document.activeElement?.matches(":focus-visible") || false,
  }))()`);
  if (!skipLinkFocus.matches || !skipLinkFocus.focusVisible) {
    throw new Error(`the first mobile Tab did not reach the visible skip link (${JSON.stringify(skipLinkFocus)})`);
  }
  const skipLink = await assertComputedAccessibleNode(page, ".skip-link", "workspace skip link", "link", "Skip to workspace");

  await dispatchKeyboardKey(page, "Tab", "Tab", 9);
  const toggleFocus = await page.evaluate(`(() => ({
    matches: document.activeElement?.id === "mobile-nav-toggle",
    focusVisible: document.activeElement?.matches(":focus-visible") || false,
    sidebarInert: document.getElementById("sidebar")?.inert === true,
  }))()`);
  if (!toggleFocus.matches || !toggleFocus.focusVisible || !toggleFocus.sidebarInert) {
    throw new Error(`the collapsed mobile sidebar intercepted keyboard focus before the menu toggle (${JSON.stringify(toggleFocus)})`);
  }
  const openToggle = await assertComputedAccessibleNode(
    page,
    "#mobile-nav-toggle",
    "closed mobile navigation toggle",
    "button",
    "Open navigation",
  );
  await dispatchKeyboardEnter(page);
  const opened = await waitForBrowserValue(
    page,
    `(() => ({
      expanded: document.getElementById("mobile-nav-toggle")?.getAttribute("aria-expanded") || "",
      focus: document.activeElement?.dataset.page || "",
      focusVisible: document.activeElement?.matches(":focus-visible") || false,
      sidebarInert: document.getElementById("sidebar")?.inert === true,
      sidebarHidden: document.getElementById("sidebar")?.getAttribute("aria-hidden") || "",
    }))()`,
    "keyboard-opened mobile navigation",
    (value) => value.expanded === "true" && value.focus === "overview",
  );
  if (opened.focus !== "overview" || !opened.focusVisible || opened.sidebarInert || opened.sidebarHidden !== "") {
    throw new Error(`opening mobile navigation did not expose and focus its active route (${JSON.stringify(opened)})`);
  }
  const closeToggle = await assertComputedAccessibleNode(
    page,
    "#mobile-nav-toggle",
    "open mobile navigation toggle",
    "button",
    "Close navigation",
  );
  const dataNavigation = await assertKeyboardFocusVisible(
    page,
    '.primary-nav [data-page="data"]',
    "Data navigation in the open mobile menu",
    "button",
    "Data",
  );
  await dispatchKeyboardEnter(page);
  const selectedData = await waitForBrowserValue(
    page,
    `(() => ({
      expanded: document.getElementById("mobile-nav-toggle")?.getAttribute("aria-expanded") || "",
      route: document.querySelector('.primary-nav [aria-current="page"][data-page]')?.dataset.page || "",
      heading: document.querySelector("#page-root h1")?.textContent.trim() || "",
      focus: document.activeElement?.id || "",
      focusVisible: document.activeElement?.matches(":focus-visible") || false,
      sidebarInert: document.getElementById("sidebar")?.inert === true,
      sidebarHidden: document.getElementById("sidebar")?.getAttribute("aria-hidden") || "",
    }))()`,
    "keyboard selection from mobile navigation",
    (value) => value.expanded === "false" && value.route === "data" && value.focus === "workspace-page-title",
  );
  if (selectedData.heading !== "Data" || !selectedData.focusVisible
    || !selectedData.sidebarInert || selectedData.sidebarHidden !== "true") {
    throw new Error(`mobile route selection did not close and inert the navigation or focus the page (${JSON.stringify(selectedData)})`);
  }

  await page.send("Page.reload", { ignoreCache: true });
  await delay(350);
  await waitForPageHeading(page);
  await dispatchKeyboardKey(page, "Tab", "Tab", 9);
  await dispatchKeyboardKey(page, "Tab", "Tab", 9);
  const escapeToggleFocus = await assertKeyboardFocusVisible(
    page,
    "#mobile-nav-toggle",
    "mobile navigation toggle before Escape path",
    "button",
    "Open navigation",
  );
  await dispatchKeyboardEnter(page);
  await waitForBrowserValue(
    page,
    'document.getElementById("mobile-nav-toggle")?.getAttribute("aria-expanded") || ""',
    "reopened mobile navigation",
    (value) => value === "true",
  );
  await dispatchKeyboardKey(page, "Escape", "Escape", 27);
  const escaped = await waitForBrowserValue(
    page,
    `(() => ({
      expanded: document.getElementById("mobile-nav-toggle")?.getAttribute("aria-expanded") || "",
      focus: document.activeElement?.id || "",
      focusVisible: document.activeElement?.matches(":focus-visible") || false,
      sidebarInert: document.getElementById("sidebar")?.inert === true,
      sidebarHidden: document.getElementById("sidebar")?.getAttribute("aria-hidden") || "",
    }))()`,
    "Escape dismissal and mobile-toggle focus restoration",
    (value) => value.expanded === "false" && value.focus === "mobile-nav-toggle",
  );
  if (!escaped.focusVisible || !escaped.sidebarInert || escaped.sidebarHidden !== "true") {
    throw new Error(`Escape did not close and inert the mobile navigation with visible focus restoration (${JSON.stringify(escaped)})`);
  }
  const restoredToggle = await assertComputedAccessibleNode(
    page,
    "#mobile-nav-toggle",
    "restored mobile navigation toggle",
    "button",
    "Open navigation",
  );

  const writes = networkRequests.filter(({ method }) => !["GET", "HEAD"].includes(method));
  if (writes.length !== 0) {
    throw new Error(`task navigation emitted an unexpected write request (${JSON.stringify(writes.map(({ method, url }) => ({ method, path: new URL(url).pathname })))})`);
  }
  return {
    status: "passed",
    chromeVersion,
    groups: observedGroups,
    routes: routeResults,
    mobile: {
      viewport: { width: 390, height: 844 },
      collapsedTabOrder: ["skip link", "navigation toggle"],
      sidebarSkippedWhileCollapsed: toggleFocus.sidebarInert,
      skipLinkAccessibleName: skipLink.name,
      openToggleName: openToggle.name,
      openFocus: opened.focus,
      closeToggleName: closeToggle.name,
      selectedRoute: selectedData.route,
      selectedRouteHeading: selectedData.heading,
      selectionFocusRestored: selectedData.focusVisible,
      escapeFocusRestored: escaped.focusVisible,
      restoredToggleName: restoredToggle.name,
    },
    writeRequests: writes.length,
  };
}

async function main() {
  const {
    retainScreenshots,
    auditContrast,
    reconciliationStdin,
    dataWorkflow,
    gpudiffScheduleWorkflow,
    dataFailureWorkflow,
    dataKeyboardViewWorkflow,
    dataFullKeyboardWorkflow,
    dataFullKeyboardAXWorkflow,
    dataFullKeyboardAXSemanticWorkflow,
    marketSupplyComparisonWorkflow,
    marketSupplyQuoteWorkflow,
    marketSupplyQuoteLifecycleWorkflow,
    marketStaleSupplyQuoteWorkflow,
    workloadTraceSemanticWorkflow,
    sessionActivityOwnerLinksWorkflow,
    acpKeyboardAXWorkflow,
    acpCancelKeyboardWorkflow,
    taskNavigationKeyboardWorkflow,
  } = parseArguments(process.argv.slice(2));
  const reconciliationInput = reconciliationStdin ? await readReconciliationStdin() : null;
  if (!fs.existsSync(serviceBinary)) {
    throw new Error("target/debug/hyperai-ui is missing; run the optional package script to build it first");
  }

  const chrome = findChrome();
  const chromeVersion = execFileSync(chrome, ["--version"], { encoding: "utf8" }).trim();
  if ((acpKeyboardAXWorkflow || acpCancelKeyboardWorkflow || taskNavigationKeyboardWorkflow)
    && chromeVersion !== "Google Chrome 154.0.8037.92") {
    throw new Error(`keyboard qualification requires the frozen Google Chrome version (${chromeVersion})`);
  }
  const servicePort = await reserveLoopbackPort();
  const devToolsPort = await reserveLoopbackPort();
  const runtimeRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hyperai-interface-browser-"));
  const home = path.join(runtimeRoot, "home");
  const profile = path.join(runtimeRoot, "chrome-profile");
  const screenshotRoot = path.join(runtimeRoot, "screenshots");
  const origin = `http://127.0.0.1:${servicePort}`;
  let service;
  let browser;
  let page;
  let browserControl;
  let retainedScreenshotDir;
  let auditReport;
  let auditReportEmitted = false;
  let cleanupEvidence;
  let browserCloseRequestStatus = "not-requested";
  let browserSignalFallbackUsed = false;
  let auditSucceeded = false;
  const browserErrors = [];
  const networkRequests = [];
  const networkResponses = [];
  const networkRequestMethods = new Map();
  const supplyQuoteRequestDigests = [];
  const quoteApprovalRequestDigests = [];
  const quoteCompletionRequestBindings = [];
  const routeResults = [];
  const contrastResults = [];
  const reducedMotionResults = [];
  let reconciliationResult;
  let dataWorkflowResult;
  let gpudiffScheduleWorkflowResult;
  let dataFailureWorkflowResult;
  let dataKeyboardViewWorkflowResult;
  let dataFullKeyboardWorkflowResult;
  let dataFullKeyboardAXWorkflowResult;
  let dataFullKeyboardAXSemanticWorkflowResult;
  let marketSupplyComparisonWorkflowResult;
  let marketSupplyQuoteWorkflowResult;
  let marketSupplyQuoteLifecycleWorkflowResult;
  let marketStaleSupplyQuoteWorkflowResult;
  let workloadTraceSemanticWorkflowResult;
  let sessionActivityOwnerLinksWorkflowResult;
  let acpKeyboardAXWorkflowResult;
  let acpCancelKeyboardWorkflowResult;
  let taskNavigationKeyboardWorkflowResult;
  let serviceListener;
  let devToolsListener;

  try {
    service = spawn(serviceBinary, ["--port", String(servicePort), "--home", home], {
      cwd: repositoryRoot,
      stdio: ["ignore", "ignore", "pipe"],
    });
    service.on("error", () => {});
    service.stderr.on("data", () => {});
    await waitForHttp(`${origin}/`, service);
    serviceListener = assertLoopbackListener(servicePort);

    browser = spawn(chrome, [
      "--headless=new",
      `--remote-debugging-address=127.0.0.1`,
      `--remote-debugging-port=${devToolsPort}`,
      `--user-data-dir=${profile}`,
      "--no-first-run",
      "--no-default-browser-check",
      "--disable-background-networking",
      "--disable-component-update",
      "--disable-sync",
      "--disable-background-mode",
      "--disable-breakpad",
      "--disable-crash-reporter",
      "--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE 127.0.0.1, EXCLUDE localhost",
      origin,
    ], {
      cwd: runtimeRoot,
      detached: process.platform !== "win32",
      stdio: ["ignore", "ignore", "ignore"],
    });
    browser.on("error", () => {});
    const target = await waitForDevTools(devToolsPort, browser);
    devToolsListener = assertLoopbackListener(devToolsPort);
    page = new DevToolsPage(target.webSocketDebuggerUrl);
    await page.opened;
    if (acpCancelKeyboardWorkflow) {
      const browserWebSocketUrl = await waitForBrowserWebSocketUrl(devToolsPort, browser);
      browserControl = new DevToolsPage(browserWebSocketUrl);
      await browserControl.opened;
    }

    page.on("Network.requestWillBeSent", ({ request, requestId }) => {
      const item = { url: request.url, method: request.method };
      networkRequests.push(item);
      networkRequestMethods.set(requestId, request.method);
      const quotePreviewPath = `${origin}/api/v1/market/supply-quote-preview`;
      if (item.url === quotePreviewPath && item.method === "POST") {
        try {
          supplyQuoteRequestDigests.push(JSON.parse(request.postData || "{}").comparison_digest || null);
        } catch {
          supplyQuoteRequestDigests.push(null);
        }
      }
      if (item.url === `${origin}/api/v1/market/quote-approve` && item.method === "POST") {
        try {
          quoteApprovalRequestDigests.push(JSON.parse(request.postData || "{}").preview_digest || null);
        } catch {
          quoteApprovalRequestDigests.push(null);
        }
      }
      if (item.url === `${origin}/api/v1/market/quote-complete` && item.method === "POST") {
        try {
          const body = JSON.parse(request.postData || "{}");
          quoteCompletionRequestBindings.push({ preview_digest: body.preview_digest || null, deal_digest: body.deal_digest || null });
        } catch {
          quoteCompletionRequestBindings.push({ preview_digest: null, deal_digest: null });
        }
      }
      const localPageRequest = item.url.startsWith(`${origin}/`);
      const authoredIcon = item.url.startsWith("data:image/svg+xml,");
      const allowedReconciliationPost = reconciliationInput !== null
        && item.method === "POST"
        && item.url === `${origin}/api/v1/inference/reconcile-execution`;
      const allowedMarketQuotePreviewPost = (marketSupplyQuoteWorkflow || marketStaleSupplyQuoteWorkflow)
        && item.method === "POST"
        && item.url === quotePreviewPath;
      const allowedMarketQuoteLifecyclePaths = new Set([
        "/api/v1/market/supply-quote-preview",
        "/api/v1/market/quote-preview/scenario/alternate",
        "/api/v1/market/quote-approve",
        "/api/v1/market/quote-complete",
      ]);
      const allowedMarketQuoteLifecyclePost = marketSupplyQuoteLifecycleWorkflow
        && item.method === "POST"
        && allowedMarketQuoteLifecyclePaths.has(new URL(item.url).pathname);
      const allowedSessionActivityPostPaths = new Set([
        "/api/v1/data/synthetic/seed",
        "/api/v1/data/query",
        "/api/v1/fleet/operation-preview",
        "/api/v1/eval/run",
        "/api/v1/trade/scenario",
        "/api/v1/trade/paper",
      ]);
      const allowedSessionActivityPost = sessionActivityOwnerLinksWorkflow
        && item.method === "POST"
        && allowedSessionActivityPostPaths.has(new URL(item.url).pathname);
      const allowedDataPostPaths = new Set([
        "/api/v1/data/synthetic/seed",
        "/api/v1/data/query",
        "/api/v1/data/models/preview",
        "/api/v1/data/models/save",
        "/api/v1/data/views/preview",
        "/api/v1/data/views/save",
      ]);
      const allowedDataPost = (dataWorkflow || dataFailureWorkflow || dataKeyboardViewWorkflow
        || dataFullKeyboardWorkflow || dataFullKeyboardAXWorkflow || dataFullKeyboardAXSemanticWorkflow)
        && item.method === "POST"
        && allowedDataPostPaths.has(new URL(item.url).pathname);
      const allowedGpudiffSchedulePostPaths = new Set([
        "/api/v1/data/gpudiff/schedule",
        "/api/v1/data/gpudiff/schedule/stop",
        "/api/v1/data/query",
      ]);
      const allowedGpudiffSchedulePost = gpudiffScheduleWorkflow
        && item.method === "POST"
        && allowedGpudiffSchedulePostPaths.has(new URL(item.url).pathname);
      const allowedACPPostPaths = new Set([
        "/api/v1/agent/connect",
        "/api/v1/agent/prompt",
        "/api/v1/agent/cancel",
        "/api/v1/agent/disconnect",
      ]);
      const allowedACPPost = (acpKeyboardAXWorkflow || acpCancelKeyboardWorkflow)
        && item.method === "POST"
        && allowedACPPostPaths.has(new URL(item.url).pathname);
      if ((!localPageRequest && !authoredIcon)
        || (!allowedReconciliationPost && !allowedMarketQuotePreviewPost && !allowedMarketQuoteLifecyclePost && !allowedDataPost && !allowedGpudiffSchedulePost && !allowedSessionActivityPost && !allowedACPPost && !["GET", "HEAD"].includes(item.method))) {
        browserErrors.push("browser attempted a nonlocal request or a non-read method");
      }
    });
    page.on("Network.responseReceived", ({ response, requestId }) => {
      networkResponses.push({
        url: response.url,
        status: response.status,
        method: networkRequestMethods.get(requestId) || "GET",
      });
    });
    page.on("Runtime.exceptionThrown", ({ exceptionDetails }) => {
      const location = exceptionDetails?.url ? new URL(exceptionDetails.url).pathname : "unknown source";
      browserErrors.push(`runtime exception at ${location}:${exceptionDetails?.lineNumber ?? 0}`);
    });
    page.on("Runtime.consoleAPICalled", ({ type, stackTrace }) => {
      if (type === "error") {
        const frame = stackTrace?.callFrames?.[0];
        const location = frame?.url ? new URL(frame.url).pathname : "unknown source";
        browserErrors.push(`console error at ${location}:${frame?.lineNumber ?? 0}`);
      }
    });
    page.on("Log.entryAdded", ({ entry }) => {
      if (entry.level === "error") {
        const location = entry.url ? new URL(entry.url).pathname : "browser";
        const source = entry.source || "unknown source";
        const message = String(entry.text || "").replaceAll(origin, "<loopback>").slice(0, 240);
        const expectedDataValidationResponse = dataWorkflow
          && source === "network"
          && location === "/api/v1/data/query"
          && /status of 422\b/.test(message)
          || dataFailureWorkflow
            && source === "network"
            && ((location === "/api/v1/data/models/preview" && /status of 409\b/.test(message))
              || (location === "/api/v1/data/views/preview" && /status of 422\b/.test(message)))
          || dataKeyboardViewWorkflow
              && source === "network"
              && location === "/api/v1/data/views/preview"
              && /status of 422\b/.test(message)
          || dataFullKeyboardWorkflow
              && source === "network"
              && location === "/api/v1/data/views/preview"
              && /status of 422\b/.test(message)
          || dataFullKeyboardAXWorkflow
              && source === "network"
              && location === "/api/v1/data/views/preview"
              && /status of 422\b/.test(message)
          || dataFullKeyboardAXSemanticWorkflow
              && source === "network"
              && location === "/api/v1/data/views/preview"
              && /status of 422\b/.test(message);
        const expectedStaleSupplyQuoteResponse = marketStaleSupplyQuoteWorkflow
          && source === "network"
          && location === "/api/v1/market/supply-quote-preview"
          && /status of 409\b/.test(message);
        if (!expectedDataValidationResponse && !expectedStaleSupplyQuoteResponse) {
          browserErrors.push(`log error at ${location} (${source}): ${message}`);
        }
      }
    });

    await page.send("Network.enable");
    await page.send("Log.enable");
    await page.send("Page.enable");
    await page.send("Runtime.enable");
    if (workloadTraceSemanticWorkflow || marketSupplyComparisonWorkflow || marketSupplyQuoteWorkflow || marketSupplyQuoteLifecycleWorkflow || marketStaleSupplyQuoteWorkflow || taskNavigationKeyboardWorkflow) {
      await page.send("Emulation.setDeviceMetricsOverride", {
        width: 1440,
        height: 1000,
        deviceScaleFactor: 1,
        mobile: false,
      });
    }
    await page.send("Page.navigate", { url: `${origin}/` });
    await waitForPageHeading(page);
    const hasAuthoredIcon = await page.evaluate(
      'document.querySelector("link[rel=icon]")?.getAttribute("href")?.startsWith("data:image/svg+xml,") === true',
    );
    if (!hasAuthoredIcon) throw new Error("the HyperAI favicon is missing or not self-contained");

    const checkedViewports = workloadTraceSemanticWorkflow || marketSupplyComparisonWorkflow || taskNavigationKeyboardWorkflow ? [] : viewports;
    for (const viewport of checkedViewports) {
      await page.send("Emulation.setDeviceMetricsOverride", {
        width: viewport.width,
        height: viewport.height,
        deviceScaleFactor: 1,
        mobile: viewport.mobile,
      });
      await delay(80);
      for (const [route, expectedName] of routes) {
        const contrastExpression = auditContrast ? contrastAuditExpression() : "null";
        const result = await page.evaluate(`(() => {
          const width = ${viewport.width};
          const route = ${JSON.stringify(route)};
          const expectedName = ${JSON.stringify(expectedName)};
          const isMobile = width <= 840;
          if (isMobile) document.getElementById("mobile-nav-toggle")?.click();
          const routeButton = document.querySelector('.primary-nav [data-page="' + route + '"]');
          routeButton?.click();
          const heading = document.querySelectorAll("#page-root h1");
          const active = document.querySelector('[aria-current="page"][data-page]');
          const activeName = active
            ? Array.from(active.childNodes)
              .filter((node) => !(node.nodeType === 1 && node.matches('[aria-hidden="true"]')))
              .map((node) => node.textContent || "")
              .join(" ")
              .replace(/\\s+/g, " ")
              .trim()
            : "";
          const controls = Array.from(document.querySelectorAll(
            'button:not(:disabled), a[href], input:not(:disabled):not([type="hidden"]), select:not(:disabled), textarea:not(:disabled)'
          )).filter((element) => !element.hidden && !element.closest('[aria-hidden="true"], [inert]'));
          const accessibleName = (element) => {
            const labelledBy = element.getAttribute("aria-labelledby");
            const labelledByText = labelledBy
              ? labelledBy.split(/\\s+/).map((id) => document.getElementById(id)?.textContent || "").join(" ").trim()
              : "";
            return labelledByText
              || element.getAttribute("aria-label")
              || element.labels?.[0]?.textContent
              || element.getAttribute("alt")
              || element.getAttribute("title")
              || element.innerText
              || element.textContent
              || "";
          };
          const overflowNodes = document.documentElement.scrollWidth > document.documentElement.clientWidth
            ? Array.from(document.body?.querySelectorAll("*") || [])
              .map((element) => {
                const rect = element.getBoundingClientRect();
                const style = getComputedStyle(element);
                return {
                  tag: element.tagName.toLowerCase(),
                  id: element.id || "",
                  classes: Array.from(element.classList).slice(0, 4),
                  role: element.getAttribute("role") || "",
                  left: Math.round(rect.left * 100) / 100,
                  right: Math.round(rect.right * 100) / 100,
                  width: Math.round(rect.width * 100) / 100,
                  minWidth: style.minWidth,
                  overflowX: style.overflowX,
                  position: style.position,
                };
              })
              .filter((element) => element.right > document.documentElement.clientWidth + 1)
              .sort((left, right) => right.right - left.right)
              .slice(0, 12)
            : [];
          const pseudoGeometry = (element, pseudo) => {
            const pseudoStyle = getComputedStyle(element, pseudo);
            if (pseudoStyle.display === "none" || pseudoStyle.content === "none"
              || pseudoStyle.content === "normal") return null;
            return {
              display: pseudoStyle.display,
              position: pseudoStyle.position,
              width: pseudoStyle.width,
              height: pseudoStyle.height,
              left: pseudoStyle.left,
              right: pseudoStyle.right,
              transform: pseudoStyle.transform,
              hasContent: pseudoStyle.content !== "" && pseudoStyle.content !== "none",
            };
          };
          const scrollCandidates = Array.from(document.body?.querySelectorAll("*") || [])
            .map((element) => {
              const rect = element.getBoundingClientRect();
              const style = getComputedStyle(element);
              return {
                element,
                tag: element.tagName.toLowerCase(),
                id: element.id || "",
                classes: Array.from(element.classList).slice(0, 4),
                left: Math.round(rect.left * 100) / 100,
                right: Math.round(rect.right * 100) / 100,
                clientWidth: element.clientWidth,
                scrollWidth: element.scrollWidth,
                overflowX: style.overflowX,
                minWidth: style.minWidth,
                position: style.position,
              };
            })
            .filter((element) => element.scrollWidth > element.clientWidth + 1)
            .sort((left, right) => (right.scrollWidth - right.clientWidth) - (left.scrollWidth - left.clientWidth))
            .slice(0, 16)
            .map(({ element, ...candidate }) => ({
              ...candidate,
              before: pseudoGeometry(element, "::before"),
              after: pseudoGeometry(element, "::after"),
            }));
          const rootGeometry = {
            viewportWidth: document.documentElement.clientWidth,
            documentElement: {
              clientWidth: document.documentElement.clientWidth,
              scrollWidth: document.documentElement.scrollWidth,
              offsetWidth: document.documentElement.offsetWidth,
            },
            body: {
              clientWidth: document.body?.clientWidth || 0,
              scrollWidth: document.body?.scrollWidth || 0,
              offsetWidth: document.body?.offsetWidth || 0,
            },
          };
          return {
            route,
            expectedName,
            heading: heading[0]?.textContent?.trim() || "",
            headingCount: heading.length,
            activeRoute: active?.dataset.page || "",
            activeName,
            focusTarget: document.activeElement?.id || "",
            focusVisible: document.activeElement?.matches(":focus-visible") || false,
            pageRootFocused: document.activeElement === document.getElementById("page-root"),
            documentWidth: document.documentElement.clientWidth,
            documentScrollWidth: document.documentElement.scrollWidth,
            overflowNodes,
            rootGeometry,
            scrollCandidates,
            unnamedControls: controls.filter((element) => !accessibleName(element).trim()).length,
            controlCount: controls.length,
            menuExpanded: document.getElementById("mobile-nav-toggle")?.getAttribute("aria-expanded") || "false",
            contrast: ${contrastExpression},
          };
        })()`);

        if (!result?.heading || result.headingCount !== 1 || result.activeRoute !== route
          || result.activeName !== expectedName) {
          throw new Error(`${viewport.name}/${route}: navigation/heading mismatch (${JSON.stringify({
            heading: result?.heading || "",
            headingCount: result?.headingCount ?? 0,
            activeRoute: result?.activeRoute || "",
            activeName: result?.activeName || "",
            expectedName,
          })})`);
        }
        if (browserErrors.length > 0) {
          throw new Error(`${browserErrors.length} browser request or runtime-policy checks failed (${browserErrors.slice(0, 5).join("; ")})`);
        }
        if (result.documentScrollWidth > result.documentWidth) {
          throw new Error(`${viewport.name}/${route}: document has horizontal overflow (${JSON.stringify({
            documentWidth: result.documentWidth,
            documentScrollWidth: result.documentScrollWidth,
            overflowNodes: result.overflowNodes,
            rootGeometry: result.rootGeometry,
            scrollCandidates: result.scrollCandidates,
          })})`);
        }
        if (result.focusTarget !== "workspace-page-title" || !result.focusVisible || result.pageRootFocused) {
          throw new Error(`${viewport.name}/${route}: navigation focus did not land visibly on the workspace heading`);
        }
        if (result.unnamedControls !== 0) {
          throw new Error(`${viewport.name}/${route}: ${result.unnamedControls} enabled controls lack accessible names`);
        }
        if (viewport.width <= 840 && result.menuExpanded !== "false") {
          throw new Error(`${viewport.name}/${route}: mobile navigation did not close after route selection`);
        }

        routeResults.push({
          viewport: viewport.name,
          width: viewport.width,
          height: viewport.height,
          route: result.route,
          heading: result.heading,
          focusTarget: result.focusTarget,
          controls: result.controlCount,
          documentWidth: result.documentWidth,
          documentScrollWidth: result.documentScrollWidth,
        });
        if (auditContrast) contrastResults.push({ viewport: viewport.name, route, ...result.contrast });

        if (screenshotTargets.has(`${viewport.name}:${route}`)) {
          await delay(120);
          const { data } = await page.send("Page.captureScreenshot", {
            format: "png",
            fromSurface: true,
            captureBeyondViewport: false,
          });
          fs.mkdirSync(screenshotRoot, { recursive: true });
          fs.writeFileSync(path.join(screenshotRoot, `${viewport.name}-${route}.png`), Buffer.from(data, "base64"));
        }
      }
    }

    if (checkedViewports.length > 0) {
      await page.send("Emulation.setEmulatedMedia", {
        features: [{ name: "prefers-reduced-motion", value: "reduce" }],
      });
    }
    for (const viewport of checkedViewports) {
      await page.send("Emulation.setDeviceMetricsOverride", {
        width: viewport.width,
        height: viewport.height,
        deviceScaleFactor: 1,
        mobile: viewport.mobile,
      });
      await delay(80);
      for (const [route, expectedName] of routes) {
        const result = await page.evaluate(`(() => {
          const route = ${JSON.stringify(route)};
          const expectedName = ${JSON.stringify(expectedName)};
          const width = ${viewport.width};
          const isMobile = width <= 840;
          if (isMobile) document.getElementById("mobile-nav-toggle")?.click();
          const routeButton = document.querySelector('.primary-nav [data-page="' + route + '"]');
          routeButton?.click();
          const heading = document.querySelectorAll("#page-root h1");
          const active = document.querySelector('[aria-current="page"][data-page]');
          const activeName = active
            ? Array.from(active.childNodes)
              .filter((node) => !(node.nodeType === 1 && node.matches('[aria-hidden="true"]')))
              .map((node) => node.textContent || "")
              .join(" ")
              .replace(/\\s+/g, " ")
              .trim()
            : "";
          const controls = Array.from(document.querySelectorAll(
            'button:not(:disabled), a[href], input:not(:disabled):not([type="hidden"]), select:not(:disabled), textarea:not(:disabled)'
          )).filter((element) => !element.hidden && !element.closest('[aria-hidden="true"], [inert]'));
          const accessibleName = (element) => {
            const labelledBy = element.getAttribute("aria-labelledby");
            const labelledByText = labelledBy
              ? labelledBy.split(/\\s+/).map((id) => document.getElementById(id)?.textContent || "").join(" ").trim()
              : "";
            return labelledByText
              || element.getAttribute("aria-label")
              || element.labels?.[0]?.textContent
              || element.getAttribute("alt")
              || element.getAttribute("title")
              || element.innerText
              || element.textContent
              || "";
          };
          const parseTimes = (value) => value.split(",").map((part) => {
            const text = part.trim();
            const number = Number.parseFloat(text);
            if (!Number.isFinite(number)) return null;
            if (text.endsWith("ms")) return number / 1000;
            if (text.endsWith("s")) return number;
            return null;
          });
          const motionViolations = [];
          const maxima = {
            transitionDurationSeconds: 0,
            transitionDelaySeconds: 0,
            animationDurationSeconds: 0,
            animationDelaySeconds: 0,
          };
          const describe = (element, pseudo) => {
            const identity = element.id ? "#" + element.id : element.tagName.toLowerCase();
            return identity + (pseudo || "");
          };
          let motionEntries = 0;
          for (const element of document.querySelectorAll("*")) {
            for (const pseudo of ["", "::before", "::after"]) {
              const style = getComputedStyle(element, pseudo || null);
              motionEntries += 1;
              const timings = [
                ["transitionDurationSeconds", style.transitionDuration, 0.000010001],
                ["transitionDelaySeconds", style.transitionDelay, 0],
                ["animationDurationSeconds", style.animationDuration, 0.000010001],
                ["animationDelaySeconds", style.animationDelay, 0],
              ];
              for (const [key, value, maximum] of timings) {
                const parsed = parseTimes(value);
                for (const seconds of parsed) {
                  if (seconds === null || seconds > maximum) {
                    if (motionViolations.length < 12) {
                      motionViolations.push({ element: describe(element, pseudo), property: key, value });
                    }
                  } else {
                    maxima[key] = Math.max(maxima[key], seconds);
                  }
                }
              }
              if (style.scrollBehavior !== "auto") {
                if (motionViolations.length < 12) {
                  motionViolations.push({ element: describe(element, pseudo), property: "scrollBehavior", value: style.scrollBehavior });
                }
              }
              const iterations = style.animationIterationCount.split(",").map(Number);
              if (iterations.some((count) => !Number.isFinite(count) || count > 1)) {
                if (motionViolations.length < 12) {
                  motionViolations.push({ element: describe(element, pseudo), property: "animationIterationCount", value: style.animationIterationCount });
                }
              }
            }
          }
          const focusTarget = document.activeElement?.id || "";
          return {
            route,
            expectedName,
            heading: heading[0]?.textContent?.trim() || "",
            headingCount: heading.length,
            activeRoute: active?.dataset.page || "",
            activeName,
            focusTarget,
            focusVisible: document.activeElement?.matches(":focus-visible") || false,
            reducedMotion: matchMedia("(prefers-reduced-motion: reduce)").matches,
            documentWidth: document.documentElement.clientWidth,
            documentScrollWidth: document.documentElement.scrollWidth,
            unnamedControls: controls.filter((element) => !accessibleName(element).trim()).length,
            menuExpanded: document.getElementById("mobile-nav-toggle")?.getAttribute("aria-expanded") || "false",
            motionEntries,
            motionViolations,
            maxima,
          };
        })()`);
        if (!result?.reducedMotion) {
          throw new Error(`${viewport.name}/${route}: reduced-motion preference was not emulated`);
        }
        if (!result.heading || result.headingCount !== 1 || result.activeRoute !== route
          || result.activeName !== expectedName) {
          throw new Error(`${viewport.name}/${route}: reduced-motion route/heading mismatch`);
        }
        if (result.documentScrollWidth > result.documentWidth || result.unnamedControls !== 0
          || result.focusTarget !== "workspace-page-title" || !result.focusVisible
          || (viewport.width <= 840 && result.menuExpanded !== "false")) {
          throw new Error(`${viewport.name}/${route}: reduced-motion navigation/layout/accessibility check failed`);
        }
        if (result.motionViolations.length > 0) {
          throw new Error(`${viewport.name}/${route}: reduced-motion CSS violation (${JSON.stringify(result.motionViolations.slice(0, 4))})`);
        }
        reducedMotionResults.push({
          viewport: viewport.name,
          width: viewport.width,
          route,
          mediaMatches: result.reducedMotion,
          motionEntries: result.motionEntries,
          maxima: result.maxima,
        });
      }
    }
    if (checkedViewports.length > 0) {
      await page.send("Emulation.setEmulatedMedia", { features: [] });
    }

    if (workloadTraceSemanticWorkflow || marketSupplyComparisonWorkflow || taskNavigationKeyboardWorkflow) {
      await page.send("Accessibility.enable");
    }
    if (taskNavigationKeyboardWorkflow) {
      taskNavigationKeyboardWorkflowResult = await exerciseTaskNavigationKeyboardWorkflow(
        page,
        origin,
        chromeVersion,
        networkRequests,
      );
    }
    if (workloadTraceSemanticWorkflow) {
      workloadTraceSemanticWorkflowResult = await exerciseWorkloadTraceSemanticWorkflow(page, chromeVersion);
      const expectedWorkloadGets = new Map([
        ["/api/v1/market/workload-trace", 2],
        ["/api/v1/market/workload-trace/failure-preview", 1],
        ["/api/v1/market/workload-trace/inconclusive-preview", 1],
      ]);
      const observedWorkloadGets = networkRequests.filter(({ method, url }) =>
        new URL(url).pathname.startsWith("/api/v1/market/workload-trace")
        && method === "GET").map(({ url }) => new URL(url).pathname);
      const observedWorkloadGetCounts = new Map();
      for (const requestPath of observedWorkloadGets) {
        observedWorkloadGetCounts.set(requestPath, (observedWorkloadGetCounts.get(requestPath) || 0) + 1);
      }
      if (observedWorkloadGets.length !== 4
        || [...expectedWorkloadGets].some(([route, count]) => observedWorkloadGetCounts.get(route) !== count)
        || [...observedWorkloadGetCounts.keys()].some((route) => !expectedWorkloadGets.has(route))) {
        throw new Error(`the Workload Trace browser workflow sent unexpected local GET requests (${JSON.stringify(Object.fromEntries(observedWorkloadGetCounts))})`);
      }
      const observedWrites = networkRequests.filter(({ method }) => !["GET", "HEAD"].includes(method));
      if (observedWrites.length > 0) {
        throw new Error(`the Workload Trace browser workflow sent a write request (${JSON.stringify(observedWrites.map(({ method, url }) => ({ method, path: new URL(url).pathname })))})`);
      }
      const responseStatuses = (requestPath) => networkResponses
        .filter(({ url }) => new URL(url).pathname === requestPath)
        .map(({ status }) => status)
        .sort((left, right) => left - right);
      const expectedWorkloadStatuses = new Map([
        ["/api/v1/market/workload-trace", [200, 200]],
        ["/api/v1/market/workload-trace/failure-preview", [200]],
        ["/api/v1/market/workload-trace/inconclusive-preview", [200]],
      ]);
      if ([...expectedWorkloadStatuses].some(([route, expected]) =>
        JSON.stringify(responseStatuses(route)) !== JSON.stringify(expected))) {
        throw new Error(`the Workload Trace browser workflow returned unexpected local statuses (${JSON.stringify(Object.fromEntries(
          [...expectedWorkloadStatuses.keys()].map((route) => [route, responseStatuses(route)]),
        ))})`);
      }
      workloadTraceSemanticWorkflowResult.localGetCounts = Object.fromEntries(observedWorkloadGetCounts);
      workloadTraceSemanticWorkflowResult.localGetStatuses = Object.fromEntries(expectedWorkloadStatuses);
      workloadTraceSemanticWorkflowResult.writeRequests = observedWrites.length;
    }

    if (marketSupplyComparisonWorkflow) {
      marketSupplyComparisonWorkflowResult = await exerciseMarketSupplyComparisonWorkflow(page, chromeVersion);
      const comparisonPath = "/api/v1/market/supply-comparison";
      const comparisonGets = networkRequests.filter(({ method, url }) =>
        method === "GET" && new URL(url).pathname === comparisonPath);
      const unexpectedComparisonMethods = networkRequests.filter(({ method, url }) =>
        new URL(url).pathname === comparisonPath && method !== "GET");
      const comparisonStatuses = networkResponses
        .filter(({ url }) => new URL(url).pathname === comparisonPath)
        .map(({ status }) => status);
      if (comparisonGets.length !== 1 || unexpectedComparisonMethods.length !== 0
        || JSON.stringify(comparisonStatuses) !== JSON.stringify([200])) {
        throw new Error("the Market comparison request contract was unexpected (" + JSON.stringify({
          gets: comparisonGets.length,
          otherMethods: unexpectedComparisonMethods.map(({ method }) => method),
          statuses: comparisonStatuses,
        }) + ")");
      }
      const observedWrites = networkRequests.filter(({ method }) => !["GET", "HEAD"].includes(method));
      if (observedWrites.length > 0) {
        throw new Error("the Market comparison workflow sent a write request (" + JSON.stringify(
          observedWrites.map(({ method, url }) => ({ method, path: new URL(url).pathname })),
        ) + ")");
      }
      marketSupplyComparisonWorkflowResult.localGetCounts = { [comparisonPath]: comparisonGets.length };
      marketSupplyComparisonWorkflowResult.localGetStatuses = { [comparisonPath]: comparisonStatuses };
      marketSupplyComparisonWorkflowResult.writeRequests = observedWrites.length;
    }

    if (marketSupplyQuoteWorkflow) {
      marketSupplyQuoteWorkflowResult = await exerciseMarketSupplyComparisonWorkflow(page, chromeVersion, true);
      const comparisonPath = "/api/v1/market/supply-comparison";
      const quotePreviewPath = "/api/v1/market/supply-quote-preview";
      const comparisonGets = networkRequests.filter(({ method, url }) =>
        method === "GET" && new URL(url).pathname === comparisonPath);
      const quotePreviewPosts = networkRequests.filter(({ method, url }) =>
        method === "POST" && new URL(url).pathname === quotePreviewPath);
      const unexpectedWrites = networkRequests.filter(({ method, url }) =>
        !["GET", "HEAD"].includes(method)
        && !(method === "POST" && new URL(url).pathname === quotePreviewPath));
      const comparisonStatuses = networkResponses
        .filter(({ url }) => new URL(url).pathname === comparisonPath)
        .map(({ status }) => status);
      const quotePreviewStatuses = networkResponses
        .filter(({ url }) => new URL(url).pathname === quotePreviewPath)
        .map(({ status }) => status);
      if (comparisonGets.length !== 1 || quotePreviewPosts.length !== 1
        || unexpectedWrites.length !== 0
        || JSON.stringify(comparisonStatuses) !== JSON.stringify([200])
        || JSON.stringify(quotePreviewStatuses) !== JSON.stringify([200])
        || JSON.stringify(supplyQuoteRequestDigests)
          !== JSON.stringify([marketSupplyQuoteWorkflowResult.comparison.comparisonDigest])) {
        throw new Error("the Market supply-to-quote request contract was unexpected (" + JSON.stringify({
          comparisonGets: comparisonGets.length,
          quotePreviewPosts: quotePreviewPosts.length,
          unexpectedWrites: unexpectedWrites.map(({ method, url }) => ({ method, path: new URL(url).pathname })),
          comparisonStatuses,
          quotePreviewStatuses,
          supplyQuoteRequestDigests,
        }) + ")");
      }
      marketSupplyQuoteWorkflowResult.localRequests = {
        [comparisonPath]: { method: "GET", status: comparisonStatuses[0] },
        [quotePreviewPath]: { method: "POST", status: quotePreviewStatuses[0] },
      };
      marketSupplyQuoteWorkflowResult.otherWrites = unexpectedWrites.length;
      marketSupplyQuoteWorkflowResult.submittedComparisonDigest = supplyQuoteRequestDigests[0];
    }

    if (marketSupplyQuoteLifecycleWorkflow) {
      marketSupplyQuoteLifecycleWorkflowResult = await exerciseMarketSupplyComparisonWorkflow(page, chromeVersion, true, true);
      const paginationSeed = await page.evaluate(`(async () => {
        const sessionResponse = await fetch("/api/v1/session", { credentials: "same-origin" });
        const session = await sessionResponse.json();
        const headers = { "Content-Type": "application/json", "X-CSRF-Token": session.csrf_token };
        const previewResponse = await fetch("/api/v1/market/quote-preview/scenario/alternate", {
          method: "POST", credentials: "same-origin", headers, body: "{}",
        });
        const preview = await previewResponse.json();
        const approvalResponse = await fetch("/api/v1/market/quote-approve", {
          method: "POST", credentials: "same-origin", headers,
          body: JSON.stringify({ preview_digest: preview.digest }),
        });
        const approval = await approvalResponse.json();
        return {
          previewStatus: previewResponse.status,
          approvalStatus: approvalResponse.status,
          previewDigest: preview.digest || "",
          dealDigest: approval.deal_digest || "",
        };
      })()`);
      if (paginationSeed.previewStatus !== 200
        || paginationSeed.approvalStatus !== 200
        || !/^[0-9a-f]{64}$/.test(paginationSeed.previewDigest)
        || !/^[0-9a-f]{64}$/.test(paginationSeed.dealDigest)
        || paginationSeed.dealDigest === marketSupplyQuoteLifecycleWorkflowResult.buyerLifecycle?.approval.dealDigest) {
        throw new Error("the alternate fixed synthetic history fixture did not produce a distinct approved deal (" + JSON.stringify(paginationSeed) + ")");
      }

      await page.send("Page.addScriptToEvaluateOnNewDocument", {
        source: `(() => {
          const originalFetch = window.fetch.bind(window);
          window.fetch = (input, init) => {
            if (typeof input === "string") {
              const url = new URL(input, window.location.href);
              if (url.pathname === "/api/v1/market/deal-history" && url.searchParams.get("limit") === "20") {
                url.searchParams.set("limit", "1");
                input = url.toString();
              }
            }
            return originalFetch(input, init);
          };
        })();`,
      });
      await page.send("Page.reload", { ignoreCache: true });
      await delay(350);
      await waitForPageHeading(page);
      const dealsNavigation = await assertKeyboardFocusVisible(
        page,
        '.primary-nav [data-page="deals"]',
        "Deals navigation after history refresh",
        "button",
        "Deals",
      );
      await dispatchKeyboardEnter(page);
      if (!dealsNavigation.focusVisible) throw new Error("Deals navigation did not retain visible keyboard focus");
      const firstHistoryPage = await waitForBrowserValue(
        page,
        '(() => { const root = document.querySelector("#page-root"); const rows = Array.from(root?.querySelectorAll(".market-history-row") || []); return { heading: root?.querySelector("#workspace-page-title")?.textContent.trim() || "", rows: rows.length, more: Boolean(root?.querySelector("[data-action=load-more-market-history]")), pageDigests: root?.querySelectorAll(".market-history-footer code").length || 0 }; })()',
        "the first bounded browser history page",
        (value) => value.heading === "Deals" && value.rows === 1 && value.more,
      );
      await page.evaluate('document.querySelector("[data-action=load-more-market-history]")?.click()');
      const traversedHistory = await waitForBrowserValue(
        page,
        '(() => { const root = document.querySelector("#page-root"); const rows = Array.from(root?.querySelectorAll(".market-history-row") || []); const digests = Array.from(root?.querySelectorAll(".market-history-row .detail-list > div") || []).filter((row) => row.querySelector("dt")?.textContent.trim() === "Deal digest").map((row) => row.querySelector("dd code")?.getAttribute("title") || ""); const pageDigests = Array.from(root?.querySelectorAll(".market-history-footer code") || []).map((node) => node.getAttribute("title") || node.textContent.trim()); return { heading: root?.querySelector("#workspace-page-title")?.textContent.trim() || "", rows: rows.length, dealDigests: digests, pageDigests, more: Boolean(root?.querySelector("[data-action=load-more-market-history]")), statuses: rows.map((row) => row.querySelector(".tag")?.textContent.trim() || "") }; })()',
        "the browser Load more cursor traversal",
        (value) => value.heading === "Deals" && value.rows === 2 && value.pageDigests.length === 2 && !value.more,
      );
      const baselineDealDigest = marketSupplyQuoteLifecycleWorkflowResult.buyerLifecycle.approval.dealDigest;
      if (!firstHistoryPage.more
        || traversedHistory.dealDigests.length !== 2
        || !traversedHistory.dealDigests.includes(baselineDealDigest)
        || !traversedHistory.dealDigests.includes(paginationSeed.dealDigest)
        || traversedHistory.pageDigests.some((digest) => !/^[0-9a-f]{64}$/.test(digest))
        || JSON.stringify(traversedHistory.statuses) !== JSON.stringify(["APPROVED LOCALLY", "MOCK COMPLETED"])) {
        throw new Error("browser Market history did not append both records and their page digests (" + JSON.stringify({ firstHistoryPage, traversedHistory, paginationSeed }) + ")");
      }
      marketSupplyQuoteLifecycleWorkflowResult.pagination = {
        firstPageRows: firstHistoryPage.rows,
        finalRows: traversedHistory.rows,
        dealDigests: traversedHistory.dealDigests,
        pageDigests: traversedHistory.pageDigests,
        statuses: traversedHistory.statuses,
        alternatePreviewDigest: paginationSeed.previewDigest,
      };

      const expectedMethods = new Map([
        ["/api/v1/market/supply-comparison", "GET"],
        ["/api/v1/market/supply-quote-preview", "POST"],
        ["/api/v1/market/quote-preview/scenario/alternate", "POST"],
        ["/api/v1/market/quote-approve", "POST"],
        ["/api/v1/market/quote-complete", "POST"],
        ["/api/v1/market/deal-history", "GET"],
      ]);
      const requestCounts = Object.fromEntries([...expectedMethods].map(([route, method]) => [route,
        networkRequests.filter((request) => request.method === method && new URL(request.url).pathname === route).length]));
      const unexpectedWrites = networkRequests.filter(({ method, url }) =>
        !["GET", "HEAD"].includes(method)
        && ![...expectedMethods].some(([route, expectedMethod]) => method === expectedMethod && new URL(url).pathname === route));
      const responseStatuses = Object.fromEntries([...expectedMethods.keys()].map((route) => [route,
        networkResponses.filter(({ url }) => new URL(url).pathname === route).map(({ status }) => status)]));
      const quotePreviewDigest = marketSupplyQuoteLifecycleWorkflowResult.quotePreview.quotePreviewDigest;
      const dealDigest = marketSupplyQuoteLifecycleWorkflowResult.buyerLifecycle?.approval.dealDigest;
      const expectedCounts = {
        ...requestCounts,
        "/api/v1/market/quote-preview/scenario/alternate": 1,
        "/api/v1/market/quote-approve": 2,
        "/api/v1/market/deal-history": 5,
      };
      if (Object.entries(expectedCounts).some(([route, count]) => requestCounts[route] !== count)
        || unexpectedWrites.length !== 0
        || [...expectedMethods.keys()].some((route) =>
          JSON.stringify(responseStatuses[route]) !== JSON.stringify(Array(expectedCounts[route]).fill(200)))
        || JSON.stringify(supplyQuoteRequestDigests) !== JSON.stringify([marketSupplyQuoteLifecycleWorkflowResult.comparison.comparisonDigest])
        || JSON.stringify(quoteApprovalRequestDigests) !== JSON.stringify([quotePreviewDigest, paginationSeed.previewDigest])
        || JSON.stringify(quoteCompletionRequestBindings) !== JSON.stringify([{ preview_digest: quotePreviewDigest, deal_digest: dealDigest }])
        || marketSupplyQuoteLifecycleWorkflowResult.buyerLifecycle?.completion.receiptDigest.length !== 64) {
        throw new Error("the staged quote-to-receipt request contract was unexpected (" + JSON.stringify({
          requestCounts,
          responseStatuses,
          unexpectedWrites: unexpectedWrites.map(({ method, url }) => ({ method, path: new URL(url).pathname })),
          supplyQuoteRequestDigests,
          quoteApprovalRequestDigests,
          quoteCompletionRequestBindings,
          quotePreviewDigest,
          dealDigest,
        }) + ")");
      }
      marketSupplyQuoteLifecycleWorkflowResult.localRequests = Object.fromEntries([...expectedMethods].map(([route, method]) => [route, {
        method,
        status: responseStatuses[route][0],
      }]));
      marketSupplyQuoteLifecycleWorkflowResult.requestCounts = requestCounts;
      marketSupplyQuoteLifecycleWorkflowResult.expectedRequestCounts = expectedCounts;
      marketSupplyQuoteLifecycleWorkflowResult.otherWrites = unexpectedWrites.length;
    }

    if (marketStaleSupplyQuoteWorkflow) {
      marketStaleSupplyQuoteWorkflowResult = await exerciseMarketSupplyComparisonWorkflow(page, chromeVersion, true, false, true);
      const expectedMethods = new Map([
        ["/api/v1/market/supply-comparison", "GET"],
        ["/api/v1/market/supply-quote-preview", "POST"],
        ["/api/v1/market/quote-approve", "POST"],
        ["/api/v1/market/quote-complete", "POST"],
      ]);
      const requestCounts = Object.fromEntries([...expectedMethods].map(([route, method]) => [route,
        networkRequests.filter((request) => request.method === method && new URL(request.url).pathname === route).length]));
      const unexpectedWrites = networkRequests.filter(({ method, url }) =>
        !["GET", "HEAD"].includes(method)
        && !(method === "POST" && new URL(url).pathname === "/api/v1/market/supply-quote-preview"));
      const responseStatuses = Object.fromEntries([...expectedMethods.keys()].map((route) => [route,
        networkResponses.filter(({ url }) => new URL(url).pathname === route).map(({ status }) => status)]));
      const staleDigest = "0".repeat(64);
      if (requestCounts["/api/v1/market/supply-comparison"] !== 1
        || requestCounts["/api/v1/market/supply-quote-preview"] !== 1
        || requestCounts["/api/v1/market/quote-approve"] !== 0
        || requestCounts["/api/v1/market/quote-complete"] !== 0
        || JSON.stringify(responseStatuses["/api/v1/market/supply-comparison"]) !== JSON.stringify([200])
        || JSON.stringify(responseStatuses["/api/v1/market/supply-quote-preview"]) !== JSON.stringify([409])
        || responseStatuses["/api/v1/market/quote-approve"].length !== 0
        || responseStatuses["/api/v1/market/quote-complete"].length !== 0
        || JSON.stringify(supplyQuoteRequestDigests) !== JSON.stringify([staleDigest])
        || marketStaleSupplyQuoteWorkflowResult.staleFailure.interception.staleDigest !== staleDigest
        || unexpectedWrites.length !== 0) {
        throw new Error("the stale supply-comparison quote failure contract was unexpected (" + JSON.stringify({
          requestCounts,
          responseStatuses,
          supplyQuoteRequestDigests,
          unexpectedWrites: unexpectedWrites.map(({ method, url }) => ({ method, path: new URL(url).pathname })),
        }) + ")");
      }
      marketStaleSupplyQuoteWorkflowResult.localRequests = Object.fromEntries([...expectedMethods].map(([route, method]) => [route, {
        method,
        statuses: responseStatuses[route],
        count: requestCounts[route],
      }]));
      marketStaleSupplyQuoteWorkflowResult.otherWrites = unexpectedWrites.length;
    }

    if (reconciliationInput) {
      reconciliationResult = await exerciseReconciliationFlow(page, reconciliationInput);
      const reconciliationPosts = networkRequests.filter(({ url, method }) =>
        method === "POST" && url === `${origin}/api/v1/inference/reconcile-execution`);
      if (reconciliationPosts.length !== 1) {
        throw new Error(`the populated browser flow sent ${reconciliationPosts.length} reconciliation requests`);
      }
    }

    if (sessionActivityOwnerLinksWorkflow) {
      sessionActivityOwnerLinksWorkflowResult = await exerciseSessionActivityOwnerLinksWorkflow(page, networkRequests, networkResponses);
    }

    if (dataWorkflow) {
      dataWorkflowResult = await exerciseDataWorkflow(page);
      const syntheticReleaseStatuses = networkResponses
        .filter(({ url }) => new URL(url).pathname === "/api/v1/data/synthetic-release")
        .map(({ status }) => status);
      if (syntheticReleaseStatuses.length === 0
        || syntheticReleaseStatuses.some((status) => status !== 200)) {
        throw new Error(`the synthetic-release projection returned unexpected statuses (${JSON.stringify(syntheticReleaseStatuses)})`);
      }
      dataWorkflowResult.derivedPipeline.syntheticReleaseStatuses = syntheticReleaseStatuses;
      const rejectedQueryResponses = networkResponses.filter(({ url, status }) =>
        new URL(url).pathname === "/api/v1/data/query" && status === 422);
      if (rejectedQueryResponses.length !== 1) {
        throw new Error(`the rejected Data query returned ${rejectedQueryResponses.length} HTTP 422 responses instead of one`);
      }
      const expectedDataPosts = new Map([
        ["/api/v1/data/synthetic/seed", 1],
        ["/api/v1/data/query", 3],
        ["/api/v1/data/models/preview", 1],
        ["/api/v1/data/models/save", 1],
        ["/api/v1/data/views/preview", 1],
        ["/api/v1/data/views/save", 1],
      ]);
      const observedDataPosts = networkRequests
        .filter(({ method, url }) => method === "POST" && url.startsWith(`${origin}/api/v1/data/`))
        .map(({ url }) => new URL(url).pathname);
      const observedDataPostCounts = new Map();
      for (const requestPath of observedDataPosts) {
        observedDataPostCounts.set(requestPath, (observedDataPostCounts.get(requestPath) || 0) + 1);
      }
      if (observedDataPosts.length !== 8
        || [...expectedDataPosts].some(([route, count]) => observedDataPostCounts.get(route) !== count)
        || [...observedDataPostCounts.keys()].some((route) => !expectedDataPosts.has(route))) {
        throw new Error(`the populated Data workflow sent unexpected POST requests (${JSON.stringify(Object.fromEntries(observedDataPostCounts))})`);
      }
      if (retainScreenshots) {
        const { data } = await page.send("Page.captureScreenshot", {
          format: "png",
          fromSurface: true,
          captureBeyondViewport: false,
        });
        fs.mkdirSync(screenshotRoot, { recursive: true });
        fs.writeFileSync(path.join(screenshotRoot, "desktop-data-workflow.png"), Buffer.from(data, "base64"));
      }
    }

    if (gpudiffScheduleWorkflow) {
      gpudiffScheduleWorkflowResult = await exerciseGpudiffScheduleWorkflow(page);
      const expectedPosts = new Map([
        ["/api/v1/data/gpudiff/schedule", 1],
        ["/api/v1/data/gpudiff/schedule/stop", 1],
        ["/api/v1/data/query", 1],
      ]);
      const observedPosts = networkRequests
        .filter(({ method, url }) => method === "POST" && url.startsWith(`${origin}/api/v1/data/`))
        .map(({ url }) => new URL(url).pathname);
      const observedCounts = new Map();
      for (const requestPath of observedPosts) {
        observedCounts.set(requestPath, (observedCounts.get(requestPath) || 0) + 1);
      }
      const responseStatuses = Object.fromEntries([...expectedPosts.keys()].map((route) => [route,
        networkResponses.filter(({ url, method }) => method === "POST" && new URL(url).pathname === route)
          .map(({ status }) => status)]));
      if (observedPosts.length !== 3
        || [...expectedPosts].some(([route, count]) => observedCounts.get(route) !== count)
        || [...observedCounts.keys()].some((route) => !expectedPosts.has(route))
        || Object.values(responseStatuses).some((statuses) => JSON.stringify(statuses) !== JSON.stringify([200]))) {
        throw new Error(`the owner GPUDiff schedule browser flow had unexpected writes or API statuses (${JSON.stringify({ observedPosts, responseStatuses })})`);
      }
      gpudiffScheduleWorkflowResult.localResponses = responseStatuses;
      gpudiffScheduleWorkflowResult.ownerWrites = observedPosts.length;
    }

    if (dataFailureWorkflow) {
      dataFailureWorkflowResult = await exerciseDataFailureWorkflow(page, home);
      const expectedDataPosts = new Map([
        ["/api/v1/data/synthetic/seed", 1],
        ["/api/v1/data/query", 3],
        ["/api/v1/data/models/preview", 3],
        ["/api/v1/data/models/save", 1],
        ["/api/v1/data/views/preview", 3],
        ["/api/v1/data/views/save", 1],
      ]);
      const observedDataPosts = networkRequests
        .filter(({ method, url }) => method === "POST" && url.startsWith(`${origin}/api/v1/data/`))
        .map(({ url }) => new URL(url).pathname);
      const observedDataPostCounts = new Map();
      for (const requestPath of observedDataPosts) {
        observedDataPostCounts.set(requestPath, (observedDataPostCounts.get(requestPath) || 0) + 1);
      }
      if (observedDataPosts.length !== 12
        || [...expectedDataPosts].some(([route, count]) => observedDataPostCounts.get(route) !== count)
        || [...observedDataPostCounts.keys()].some((route) => !expectedDataPosts.has(route))) {
        throw new Error(`the Data failure workflow sent unexpected POST requests (${JSON.stringify(Object.fromEntries(observedDataPostCounts))})`);
      }
      const responseStatuses = (requestPath) => networkResponses
        .filter(({ url }) => new URL(url).pathname === requestPath)
        .map(({ status }) => status)
        .sort((left, right) => left - right);
      const expectedStatuses = new Map([
        ["/api/v1/data/synthetic/seed", [200]],
        ["/api/v1/data/query", [200, 200, 200]],
        ["/api/v1/data/models/preview", [200, 200, 409]],
        ["/api/v1/data/models/save", [200]],
        ["/api/v1/data/views/preview", [200, 200, 422]],
        ["/api/v1/data/views/save", [200]],
      ]);
      if ([...expectedStatuses].some(([route, expected]) =>
        JSON.stringify(responseStatuses(route)) !== JSON.stringify(expected))) {
        throw new Error(`the Data failure workflow returned unexpected API statuses (${JSON.stringify(Object.fromEntries(
          [...expectedStatuses.keys()].map((route) => [route, responseStatuses(route)]),
        ))})`);
      }
      if (retainScreenshots) {
        const { data } = await page.send("Page.captureScreenshot", {
          format: "png",
          fromSurface: true,
          captureBeyondViewport: false,
        });
        fs.mkdirSync(screenshotRoot, { recursive: true });
        fs.writeFileSync(path.join(screenshotRoot, "desktop-data-failure-workflow.png"), Buffer.from(data, "base64"));
      }
    }

    if (dataKeyboardViewWorkflow) {
      dataKeyboardViewWorkflowResult = await exerciseDataKeyboardViewWorkflow(page);
      const expectedDataPosts = new Map([
        ["/api/v1/data/synthetic/seed", 1],
        ["/api/v1/data/query", 1],
        ["/api/v1/data/models/preview", 1],
        ["/api/v1/data/models/save", 1],
        ["/api/v1/data/views/preview", 2],
        ["/api/v1/data/views/save", 1],
      ]);
      const observedDataPosts = networkRequests
        .filter(({ method, url }) => method === "POST" && url.startsWith(`${origin}/api/v1/data/`))
        .map(({ url }) => new URL(url).pathname);
      const observedDataPostCounts = new Map();
      for (const requestPath of observedDataPosts) {
        observedDataPostCounts.set(requestPath, (observedDataPostCounts.get(requestPath) || 0) + 1);
      }
      if (observedDataPosts.length !== 7
        || [...expectedDataPosts].some(([route, count]) => observedDataPostCounts.get(route) !== count)
        || [...observedDataPostCounts.keys()].some((route) => !expectedDataPosts.has(route))) {
        throw new Error(`the Data keyboard workflow sent unexpected POST requests (${JSON.stringify(Object.fromEntries(observedDataPostCounts))})`);
      }
      const responseStatuses = (requestPath) => networkResponses
        .filter(({ url }) => new URL(url).pathname === requestPath)
        .map(({ status }) => status)
        .sort((left, right) => left - right);
      const expectedStatuses = new Map([
        ["/api/v1/data/synthetic/seed", [200]],
        ["/api/v1/data/query", [200]],
        ["/api/v1/data/models/preview", [200]],
        ["/api/v1/data/models/save", [200]],
        ["/api/v1/data/views/preview", [200, 422]],
        ["/api/v1/data/views/save", [200]],
      ]);
      if ([...expectedStatuses].some(([route, expected]) =>
        JSON.stringify(responseStatuses(route)) !== JSON.stringify(expected))) {
        throw new Error(`the Data keyboard workflow returned unexpected API statuses (${JSON.stringify(Object.fromEntries(
          [...expectedStatuses.keys()].map((route) => [route, responseStatuses(route)]),
        ))})`);
      }
    }

    if (dataFullKeyboardWorkflow || dataFullKeyboardAXWorkflow || dataFullKeyboardAXSemanticWorkflow) {
      const nameComparison = dataFullKeyboardAXSemanticWorkflow
        ? "nfc-whitespace-ascii-case-insensitive"
        : "exact";
      const fullKeyboardResult = await exerciseDataFullKeyboardWorkflow(page, origin, nameComparison);
      if (dataFullKeyboardAXWorkflow) dataFullKeyboardAXWorkflowResult = fullKeyboardResult;
      else if (dataFullKeyboardAXSemanticWorkflow) dataFullKeyboardAXSemanticWorkflowResult = fullKeyboardResult;
      else dataFullKeyboardWorkflowResult = fullKeyboardResult;
      const expectedDataPosts = new Map([
        ["/api/v1/data/synthetic/seed", 1],
        ["/api/v1/data/query", 1],
        ["/api/v1/data/models/preview", 1],
        ["/api/v1/data/models/save", 1],
        ["/api/v1/data/views/preview", 2],
        ["/api/v1/data/views/save", 1],
      ]);
      const observedDataPosts = networkRequests
        .filter(({ method, url }) => method === "POST" && url.startsWith(`${origin}/api/v1/data/`))
        .map(({ url }) => new URL(url).pathname);
      const observedDataPostCounts = new Map();
      for (const requestPath of observedDataPosts) {
        observedDataPostCounts.set(requestPath, (observedDataPostCounts.get(requestPath) || 0) + 1);
      }
      if (observedDataPosts.length !== 7
        || [...expectedDataPosts].some(([route, count]) => observedDataPostCounts.get(route) !== count)
        || [...observedDataPostCounts.keys()].some((route) => !expectedDataPosts.has(route))) {
        throw new Error(`the full Data keyboard workflow sent unexpected POST requests (${JSON.stringify(Object.fromEntries(observedDataPostCounts))})`);
      }
      const responseStatuses = (requestPath) => networkResponses
        .filter(({ url }) => new URL(url).pathname === requestPath)
        .map(({ status }) => status)
        .sort((left, right) => left - right);
      const expectedStatuses = new Map([
        ["/api/v1/data/synthetic/seed", [200]],
        ["/api/v1/data/query", [200]],
        ["/api/v1/data/models/preview", [200]],
        ["/api/v1/data/models/save", [200]],
        ["/api/v1/data/views/preview", [200, 422]],
        ["/api/v1/data/views/save", [200]],
      ]);
      if ([...expectedStatuses].some(([route, expected]) =>
        JSON.stringify(responseStatuses(route)) !== JSON.stringify(expected))) {
        throw new Error(`the full Data keyboard workflow returned unexpected API statuses (${JSON.stringify(Object.fromEntries(
          [...expectedStatuses.keys()].map((route) => [route, responseStatuses(route)]),
        ))})`);
      }
    }

    if (acpKeyboardAXWorkflow || acpCancelKeyboardWorkflow) {
      const result = await exerciseACPKeyboardAXWorkflow(page, origin, {
        cancellation: acpCancelKeyboardWorkflow,
      });
      if (acpCancelKeyboardWorkflow) acpCancelKeyboardWorkflowResult = result;
      else acpKeyboardAXWorkflowResult = result;
      const expectedACPPosts = new Map([
        ["/api/v1/agent/connect", 1],
        ["/api/v1/agent/prompt", 1],
        ...(acpCancelKeyboardWorkflow ? [["/api/v1/agent/cancel", 1]] : []),
        ["/api/v1/agent/disconnect", 1],
      ]);
      const observedACPPosts = networkRequests
        .filter(({ method, url }) => method === "POST" && url.startsWith(`${origin}/api/v1/agent/`))
        .map(({ url }) => new URL(url).pathname);
      const observedACPPostCounts = new Map();
      for (const requestPath of observedACPPosts) {
        observedACPPostCounts.set(requestPath, (observedACPPostCounts.get(requestPath) || 0) + 1);
      }
      if (observedACPPosts.length !== expectedACPPosts.size
        || [...expectedACPPosts].some(([route, count]) => observedACPPostCounts.get(route) !== count)
        || [...observedACPPostCounts.keys()].some((route) => !expectedACPPosts.has(route))) {
        throw new Error(`the ACP keyboard workflow sent unexpected POST requests (${JSON.stringify(Object.fromEntries(observedACPPostCounts))})`);
      }
      const responseStatuses = (requestPath) => networkResponses
        .filter(({ url }) => new URL(url).pathname === requestPath)
        .map(({ status }) => status)
        .sort((left, right) => left - right);
      if ([...expectedACPPosts.keys()].some((route) =>
        JSON.stringify(responseStatuses(route)) !== JSON.stringify([200]))) {
        throw new Error(`the ACP keyboard workflow returned unexpected API statuses (${JSON.stringify(Object.fromEntries(
          [...expectedACPPosts.keys()].map((route) => [route, responseStatuses(route)]),
        ))})`);
      }
    }

    await delay(250);
    if (browserErrors.length > 0) {
      throw new Error(`${browserErrors.length} browser request or runtime-policy checks failed (${browserErrors.slice(0, 5).join("; ")})`);
    }
    if (networkRequests.length === 0) {
      throw new Error("the browser audit did not observe the local page requests");
    }
    if (networkRequests.some(({ url }) => url === `${origin}/favicon.ico`)) {
      throw new Error("the browser requested the missing favicon fallback");
    }

    if (retainScreenshots && fs.existsSync(screenshotRoot)) {
      retainedScreenshotDir = path.join(os.tmpdir(), `hyperai-interface-screenshots-${randomUUID()}`);
      fs.mkdirSync(retainedScreenshotDir, { mode: 0o700 });
      for (const name of fs.readdirSync(screenshotRoot)) {
        fs.copyFileSync(path.join(screenshotRoot, name), path.join(retainedScreenshotDir, name));
      }
    }
    const contrastFailureCount = contrastResults.reduce((total, result) => total + result.failureCount, 0);
    const contrastSamplesChecked = contrastResults.reduce((total, result) => total + result.checked, 0);
    const contrastReviewRequired = contrastResults.reduce((total, result) => total + result.reviewed, 0);
    const contrastFailureGroups = new Map();
    for (const { viewport, route, failures } of contrastResults) {
      for (const failure of failures) {
        const key = JSON.stringify([failure.foreground, failure.background, failure.threshold]);
        const group = contrastFailureGroups.get(key) || {
          selectors: new Set(),
          text: failure.text,
          ratio: failure.ratio,
          threshold: failure.threshold,
          foreground: failure.foreground,
          background: failure.background,
          occurrences: 0,
          routes: new Set(),
          viewports: new Set(),
          examples: new Set(),
        };
        group.occurrences += 1;
        if (group.selectors.size < 5) group.selectors.add(failure.selector);
        group.routes.add(route);
        group.viewports.add(viewport);
        if (group.examples.size < 3) group.examples.add(failure.text);
        if (failure.ratio < group.ratio) {
          group.ratio = failure.ratio;
          group.text = failure.text;
        }
        contrastFailureGroups.set(key, group);
      }
    }
    const contrastFailures = [...contrastFailureGroups.values()]
      .map((group) => ({
        selectors: [...group.selectors],
        foreground: group.foreground,
        background: group.background,
        ratio: group.ratio,
        threshold: group.threshold,
        occurrences: group.occurrences,
        routeCount: group.routes.size,
        viewports: [...group.viewports],
        example: [...group.examples][0],
      }))
      .sort((a, b) => a.ratio - b.ratio);
    const contrastReviewSamples = [...new Map(contrastResults
      .flatMap(({ viewport, route, reviewSamples }) => reviewSamples.map((sample) => [
        JSON.stringify([sample.selector, sample.reason]),
        { viewport, route, ...sample },
      ]))).values()].slice(0, 12);
    const contrastMinimum = contrastResults.reduce((minimum, result) => {
      if (!result.minimum) return minimum;
      const sample = { viewport: result.viewport, route: result.route, ...result.minimum };
      return !minimum || sample.ratio < minimum.ratio ? sample : minimum;
    }, null);
    const contrastStatus = !auditContrast
      ? "not-run"
      : contrastSamplesChecked === 0 || contrastFailureCount > 0
        ? "failed"
        : contrastReviewRequired > 0
          ? "passed-with-review-required"
          : "passed";
    auditSucceeded = true;
    auditReport = {
      status: contrastStatus === "failed" ? "contrast-failed" : "passed",
      chromeVersion,
      runnerProcesses: {
        servicePid: service?.pid ?? null,
        browserPid: browser?.pid ?? null,
        browserProcessGroupId: process.platform === "win32" ? null : browser?.pid ?? null,
      },
      ephemeralLoopbackPorts: {
        service: servicePort,
        devTools: devToolsPort,
      },
      serviceListener,
      devToolsListener,
      routesChecked: taskNavigationKeyboardWorkflowResult?.routes.length ?? routeResults.length,
      requestsChecked: networkRequests.length,
      methods: [...new Set(networkRequests.map(({ method }) => method))].sort(),
      routeResults: auditContrast ? undefined : (taskNavigationKeyboardWorkflowResult?.routes ?? routeResults),
      reducedMotion: {
        status: reducedMotionResults.length > 0 ? "passed" : "not-run",
        routeViewportsChecked: reducedMotionResults.length,
        motionEntriesChecked: reducedMotionResults.reduce((total, result) => total + result.motionEntries, 0),
        maximumObserved: reducedMotionResults.reduce((maximum, result) => ({
          transitionDurationSeconds: Math.max(maximum.transitionDurationSeconds, result.maxima.transitionDurationSeconds),
          transitionDelaySeconds: Math.max(maximum.transitionDelaySeconds, result.maxima.transitionDelaySeconds),
          animationDurationSeconds: Math.max(maximum.animationDurationSeconds, result.maxima.animationDurationSeconds),
          animationDelaySeconds: Math.max(maximum.animationDelaySeconds, result.maxima.animationDelaySeconds),
        }), {
          transitionDurationSeconds: 0,
          transitionDelaySeconds: 0,
          animationDurationSeconds: 0,
          animationDelaySeconds: 0,
        }),
      },
      reconciliation: reconciliationResult,
      dataWorkflow: dataWorkflowResult,
      gpudiffScheduleWorkflow: gpudiffScheduleWorkflowResult,
      dataFailureWorkflow: dataFailureWorkflowResult,
      dataKeyboardViewWorkflow: dataKeyboardViewWorkflowResult,
      dataFullKeyboardWorkflow: dataFullKeyboardWorkflowResult,
      dataFullKeyboardAXWorkflow: dataFullKeyboardAXWorkflowResult,
      dataFullKeyboardAXSemanticWorkflow: dataFullKeyboardAXSemanticWorkflowResult,
      marketSupplyComparisonWorkflow: marketSupplyComparisonWorkflowResult,
      marketSupplyQuoteWorkflow: marketSupplyQuoteWorkflowResult,
      marketSupplyQuoteLifecycleWorkflow: marketSupplyQuoteLifecycleWorkflowResult,
      marketStaleSupplyQuoteWorkflow: marketStaleSupplyQuoteWorkflowResult,
      workloadTraceSemanticWorkflow: workloadTraceSemanticWorkflowResult,
      sessionActivityOwnerLinksWorkflow: sessionActivityOwnerLinksWorkflowResult,
      acpKeyboardAXWorkflow: acpKeyboardAXWorkflowResult,
      acpCancelKeyboardWorkflow: acpCancelKeyboardWorkflowResult,
      taskNavigationKeyboardWorkflow: taskNavigationKeyboardWorkflowResult,
      contrast: auditContrast ? {
        status: contrastStatus,
        standard: "WCAG 2.2 SC 1.4.3",
        thresholds: { ordinaryText: 4.5, largeText: 3 },
        routeViewportsChecked: contrastResults.length,
        samplesChecked: contrastSamplesChecked,
        reviewRequired: contrastReviewRequired,
        minimum: contrastMinimum,
        failureCount: contrastFailureCount,
        failures: contrastFailures,
        reviewSamples: contrastReviewSamples,
      } : undefined,
      screenshots: retainedScreenshotDir,
    };
    if (contrastStatus === "failed") process.exitCode = 1;
  } catch (error) {
    const message = error instanceof Error ? error.message : "unexpected interface browser audit failure";
    process.stderr.write("Browser audit error before cleanup: " + message + "\n");
    throw error;
  } finally {
    page?.close();
    const cleanupErrors = [];
    if (acpCancelKeyboardWorkflow && browserControl) {
      try {
        await browserControl.send("Browser.close");
        browserCloseRequestStatus = "command-response";
      } catch (error) {
        browserCloseRequestStatus = error?.message === "DevTools WebSocket closed"
          ? "transport-closed"
          : "fallback-after-command-error";
      }
      browserControl.close();
      let gracefulExit = false;
      try {
        gracefulExit = process.platform === "win32"
          ? await waitForExit(browser, 5000)
          : await waitForProcessGroupEmpty(browser.pid, 5000);
      } catch (error) {
        cleanupErrors.push(error);
      }
      let browserScopeStillRunning = Boolean(browser?.pid) && (process.platform === "win32"
        ? !childHasExited(browser)
        : true);
      if (browser?.pid && process.platform !== "win32") {
        try {
          browserScopeStillRunning = processGroupHasLiveMembers(browser.pid);
        } catch (error) {
          cleanupErrors.push(error);
          browserScopeStillRunning = true;
        }
      }
      if (browserScopeStillRunning && process.platform !== "win32") {
        browserSignalFallbackUsed = true;
        try {
          await stopProcessGroup(browser.pid);
        } catch (error) {
          cleanupErrors.push(error);
        }
        try {
          browserScopeStillRunning = processGroupHasLiveMembers(browser.pid);
        } catch (error) {
          cleanupErrors.push(error);
          browserScopeStillRunning = true;
        }
      } else if (!gracefulExit && browserScopeStillRunning) {
        browserSignalFallbackUsed = true;
        try {
          await stopProcess(browser, false);
        } catch (error) {
          cleanupErrors.push(error);
        }
      }
    } else {
      try {
        await stopProcess(browser, process.platform !== "win32");
      } catch (error) {
        cleanupErrors.push(error);
      }
    }
    try {
      await stopProcess(service);
    } catch (error) {
      cleanupErrors.push(error);
    }

    let browserGroupMemberStates = [];
    let browserScopeRunning = Boolean(browser?.pid) && (process.platform === "win32"
      ? !childHasExited(browser)
      : false);
    if (browser?.pid && process.platform !== "win32") {
      try {
        browserGroupMemberStates = readProcessGroupMembers(browser.pid);
        browserScopeRunning = hasLiveProcessGroupMembers(browserGroupMemberStates);
      } catch (error) {
        cleanupErrors.push(error);
        browserScopeRunning = true;
      }
    }
    if (browserScopeRunning && process.platform !== "win32") {
      browserSignalFallbackUsed = true;
      try {
        await stopProcessGroup(browser.pid);
      } catch (error) {
        cleanupErrors.push(error);
      }
      try {
        browserGroupMemberStates = readProcessGroupMembers(browser.pid);
        browserScopeRunning = hasLiveProcessGroupMembers(browserGroupMemberStates);
      } catch (error) {
        cleanupErrors.push(error);
        browserScopeRunning = true;
      }
    }
    let listenerResults = [
      { name: "service", port: servicePort, closed: false },
      { name: "devTools", port: devToolsPort, closed: false },
    ];
    try {
      const closed = await Promise.all([
        loopbackPortIsClosed(servicePort),
        loopbackPortIsClosed(devToolsPort),
      ]);
      listenerResults = listenerResults.map((listener, index) => ({ ...listener, closed: closed[index] }));
    } catch (error) {
      cleanupErrors.push(error);
    }
    const listenersClosed = listenerResults.every(({ closed }) => closed);
    if (!listenersClosed) {
      cleanupErrors.push(new Error("a runner-reserved loopback listener remained open; temporary state was preserved"));
    }
    const serviceRunning = Boolean(service?.pid) && !childHasExited(service);
    const processesExited = !browserScopeRunning && !serviceRunning && listenersClosed;
    if (processesExited) {
      fs.rmSync(runtimeRoot, { recursive: true, force: true });
      if (!auditSucceeded && retainedScreenshotDir) {
        fs.rmSync(retainedScreenshotDir, { recursive: true, force: true });
      }
    } else {
      cleanupErrors.push(new Error(`runner-owned browser process group ${browser?.pid ?? "unavailable"} or service PID ${service?.pid ?? "unavailable"} remained active; temporary state was preserved`));
    }

    cleanupEvidence = {
      browserCloseRequestStatus,
      browserSignalFallbackUsed,
      browserPid: browser?.pid ?? null,
      browserProcessGroupId: process.platform === "win32" ? null : browser?.pid ?? null,
      browserExitCode: browser?.exitCode ?? null,
      browserSignalCode: browser?.signalCode ?? null,
      browserScopeExited: !browserScopeRunning,
      browserHasLiveMembers: browserScopeRunning,
      browserGroupMemberStates: browserGroupMemberStates.map(({ pid, state }) => ({ pid, state })),
      servicePid: service?.pid ?? null,
      serviceExitCode: service?.exitCode ?? null,
      serviceSignalCode: service?.signalCode ?? null,
      serviceExited: !serviceRunning,
      ephemeralListeners: listenerResults,
      listenersClosed,
      temporaryStateRemoved: processesExited && !fs.existsSync(runtimeRoot),
    };

    if (!auditReport && cleanupErrors.length > 0) {
      process.stderr.write("Browser cleanup evidence: " + JSON.stringify(cleanupEvidence) + "\n");
    }
    if (auditReport) {
      auditReport.workflowStatus = auditReport.status;
      auditReport.cleanup = cleanupEvidence;
      if (cleanupErrors.length > 0) auditReport.status = "inconclusive";
    }

    if (auditReport && acpCancelKeyboardWorkflow) {
      auditReport.workflowStatus = auditReport.status;
      auditReport.cleanup = cleanupEvidence;
      if (cleanupErrors.length > 0) auditReport.status = "inconclusive";
      process.stdout.write(`${JSON.stringify(auditReport, null, 2)}\n`);
      auditReportEmitted = true;
    }

    if (cleanupErrors.length > 0) {
      if (auditReport && !auditReportEmitted) {
        process.stdout.write(JSON.stringify(auditReport, null, 2) + "\n");
        auditReportEmitted = true;
      }
      throw cleanupErrors[0];
    }
  }

  if (auditReport && !auditReportEmitted) {
    process.stdout.write(`${JSON.stringify(auditReport, null, 2)}\n`);
  }
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : "unexpected interface browser audit failure";
  process.stderr.write(`HyperAI interface browser audit failed: ${message}\n`);
  process.exitCode = 1;
});
