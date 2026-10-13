/* Renders the versioned, static connection inventory without trusting its text. */
(function installConnectionInventoryView(root) {
  function escapeHtml(value) {
    return String(value ?? "").replace(/[&<>\"']/g, (character) => ({
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      "\"": "&quot;",
      "'": "&#39;",
    })[character]);
  }

  function label(value, fallback = "Not specified") {
    return typeof value === "string" && value.length
      ? value.replaceAll("_", " ")
      : fallback;
  }

  const workspacePageByModuleId = Object.freeze({
    compute_market: "market",
    simple_compute_market: "market",
    hyperintel: "inference",
    fleet: "fleet",
    eval: "eval",
    trade: "trade",
    data: "data",
  });
  const navigableModes = new Set(["local_synthetic", "offline_caller_selected"]);

  function workspaceAction(item) {
    const moduleId = item?.module_id;
    const page = Object.hasOwn(workspacePageByModuleId, moduleId)
      ? workspacePageByModuleId[moduleId]
      : null;
    if (!page
      || !navigableModes.has(item?.connection_mode)
      || !Array.isArray(item?.exposed_capability_ids)
      || item.exposed_capability_ids.length === 0) return "";

    const name = escapeHtml(item.name || "HyperAI");
    return `<button class="connection-open" type="button" data-page="${page}" aria-label="Open ${name} workspace">Open workspace <span aria-hidden="true">↗</span></button>`;
  }

  function renderCards(inventory, limit = Infinity) {
    const items = Array.isArray(inventory?.connections)
      ? inventory.connections.slice(0, limit)
      : [];
    if (!items.length) {
      return `<div class="connection-cards"><p class="empty-inline">Connection inventory is unavailable.</p></div>`;
    }
    return `<div class="connection-cards">${items.map((item) => `<article class="connection-card"><div><span class="connection-mark">${escapeHtml(String(item.name || "?").slice(0, 1))}</span><span class="tag tag-neutral">${escapeHtml(label(item.connection_mode))}</span></div><h3>${escapeHtml(item.name || "Unknown connection")}</h3><p>${escapeHtml(item.boundary || "Boundary unavailable.")}</p>${workspaceAction(item)}</article>`).join("")}</div>`;
  }

  function renderList(inventory) {
    const items = inventory?.schema_version === "hyperai.interface.connection-inventory.v1"
      && Array.isArray(inventory.connections)
      ? inventory.connections
      : null;
    if (!items) {
      return `<div class="connection-list"><p class="empty-inline" role="status">Connection inventory is unavailable. No external health check was attempted.</p></div>`;
    }
    const rows = items.map((item, index) => {
      const revision = typeof item.source_revision === "string" && item.source_revision
        ? `<code>${escapeHtml(item.source_revision)}</code>`
        : `<span>Not pinned</span>`;
      const capabilities = Array.isArray(item.exposed_capability_ids) && item.exposed_capability_ids.length
        ? item.exposed_capability_ids.map((id) => `<li><code>${escapeHtml(id)}</code></li>`).join("")
        : `<li class="connection-no-capabilities">No exposed surfaces in this interface</li>`;
      const gate = item.next_blocked_gate && typeof item.next_blocked_gate === "object"
        ? item.next_blocked_gate
        : {};
      return `<article class="connection-descriptor" aria-labelledby="connection-heading-${index}"><div class="connection-descriptor-heading"><div><h3 id="connection-heading-${index}">${escapeHtml(item.name || "Unknown connection")}</h3><span class="connection-module-id"><code>${escapeHtml(item.module_id || "unknown")}</code></span></div><span class="tag tag-neutral">${escapeHtml(label(item.connection_mode))}</span></div><p class="connection-boundary">${escapeHtml(item.boundary || "Boundary unavailable.")}</p><dl class="connection-descriptor-details"><div><dt>Owner</dt><dd>${escapeHtml(item.owner || "Unknown owner")}</dd></div><div><dt>State</dt><dd>${escapeHtml(label(item.state))}</dd></div><div><dt>Source revision</dt><dd>${revision}</dd></div></dl><section class="connection-capabilities" aria-label="Exposed surfaces"><h4>Exposed surfaces</h4><ul>${capabilities}</ul></section><section class="connection-next-gate"><h4>Next blocked gate <code>${escapeHtml(gate.id || "unspecified")}</code></h4><p>${escapeHtml(gate.summary || "No gate description was returned.")}</p></section>${workspaceAction(item)}</article>`;
    }).join("");
    return `<div class="connection-list" aria-label="Static HyperAI connection inventory">${rows || `<p class="empty-inline">The inventory contains no entries.</p>`}</div>`;
  }

  root.HyperAIConnectionInventoryView = { renderCards, renderList };
})(typeof window === "undefined" ? globalThis : window);
