const ROUTES = {
  "language-tour": "./language-tour.md",
  std: "./std.md",
};

const ROUTE_META = {
  "language-tour": {
    label: "Language Tour",
    summary: "The introductory path for syntax, semantics, and the current execution loop.",
  },
  std: {
    label: "Standard Surface",
    summary: "A compact map of the current builtins, capabilities, and runtime boundary.",
  },
};

const ACTIONS = ["run", "check", "build", "test"];

const state = {
  route: "language-tour",
  doc: null,
  docError: null,
  manifest: null,
  manifestError: null,
  selectedSnippetId: null,
  selectedAction: null,
};

const cache = {
  manifestPromise: null,
  docs: new Map(),
};

const els = {};

let loadToken = 0;

if (typeof document !== "undefined") {
  document.addEventListener("DOMContentLoaded", start);
}

if (typeof window !== "undefined") {
  window.addEventListener("hashchange", () => {
    void navigateFromHash();
  });
}

function start() {
  if (typeof document === "undefined") {
    return;
  }

  els.sidebar = document.getElementById("sidebar");
  els.docView = document.getElementById("doc-view");
  els.playgroundPane = document.getElementById("playground-pane");

  if (!els.sidebar || !els.docView || !els.playgroundPane) {
    return;
  }

  els.docView.addEventListener("click", handleDocClick);
  els.playgroundPane.addEventListener("click", handlePaneClick);

  void navigateFromHash();
}

async function navigateFromHash() {
  const route = resolveRouteFromHash(typeof location !== "undefined" ? location.hash : "");
  canonicalizeHash(route);

  state.route = route;
  state.doc = null;
  state.docError = null;
  state.selectedSnippetId = null;
  state.selectedAction = null;

  renderShell();

  const token = ++loadToken;

  const [docResult, manifestResult] = await Promise.allSettled([
    loadDocument(route),
    loadManifest(),
  ]);

  if (token !== loadToken) {
    return;
  }

  if (docResult.status === "fulfilled") {
    state.doc = docResult.value;
    state.docError = null;
  } else {
    state.doc = null;
    state.docError = formatError(docResult.reason, "Document failed to load.");
  }

  if (manifestResult.status === "fulfilled") {
    state.manifest = manifestResult.value;
    state.manifestError = null;
  } else {
    state.manifest = null;
    state.manifestError = formatError(
      manifestResult.reason,
      "Manifest failed to load. Snippet actions are disabled."
    );
  }

  if (state.doc) {
    document.title = `${state.doc.title} · Arukellt Docs`;
  } else {
    document.title = `Arukellt Docs`;
  }

  renderShell();
}

function resolveRouteFromHash(hash) {
  const raw = String(hash || "").replace(/^#\/?/, "").split(/[?#]/)[0].trim();
  if (raw && ROUTES[raw]) {
    return raw;
  }
  return "language-tour";
}

function canonicalizeHash(route) {
  if (typeof location === "undefined" || typeof history === "undefined") {
    return;
  }

  const canonical = `#/${route}`;
  if (location.hash !== canonical) {
    history.replaceState(null, "", canonical);
  }
}

function renderShell() {
  renderSidebar();
  renderMain();
  renderPane();
}

function renderSidebar() {
  const routeMeta = ROUTE_META[state.route] || ROUTE_META["language-tour"];

  els.sidebar.innerHTML = `
    <div class="brand-mark">Arukellt Docs</div>
    <p class="project-summary">Static docs shell for the language tour and standard surface, served directly from GitHub Pages.</p>
    <nav class="nav-group" aria-label="Documentation sections">
      <a class="nav-link" href="#/language-tour" ${state.route === "language-tour" ? 'aria-current="page"' : ""}>
        <span class="nav-label">Language Tour</span>
        <span class="nav-caption">${escapeHtml(ROUTE_META["language-tour"].summary)}</span>
      </a>
      <a class="nav-link" href="#/std" ${state.route === "std" ? 'aria-current="page"' : ""}>
        <span class="nav-label">Standard Surface</span>
        <span class="nav-caption">${escapeHtml(ROUTE_META.std.summary)}</span>
      </a>
      <span class="nav-placeholder">
        <span class="nav-label">Examples</span>
        <span class="nav-caption">Placeholder</span>
      </span>
      <span class="nav-placeholder">
        <span class="nav-label">Reference</span>
        <span class="nav-caption">Placeholder</span>
      </span>
    </nav>
  `;

  void routeMeta;
}

function renderMain() {
  if (state.docError) {
    els.docView.innerHTML = `
      <section class="loading-state">
        <div class="pane-card">
          <p class="doc-kicker">Document error</p>
          <h1 class="pane-title">Unable to load the selected document</h1>
          <p class="project-summary">${escapeHtml(state.docError)}</p>
        </div>
      </section>
    `;
    return;
  }

  if (!state.doc) {
    els.docView.innerHTML = `
      <section class="loading-state" aria-label="Loading document">
        <div class="loading-line loading-line--wide"></div>
        <div class="loading-line loading-line--medium"></div>
        <div class="loading-line loading-line--short"></div>
        <div class="loading-line loading-line--wide"></div>
      </section>
    `;
    return;
  }

  const summary = state.doc.summary || ROUTE_META[state.route].summary;
  const content = state.doc.blocks.map((block) => renderBlock(block)).join("");

  els.docView.innerHTML = `
    <section class="doc-surface">
      <header class="doc-hero">
        <p class="doc-kicker">Documentation</p>
        <h1 class="doc-title">${escapeHtml(state.doc.title)}</h1>
        <p class="doc-summary">${escapeHtml(summary)}</p>
      </header>
      <article class="doc-content">
        ${content}
      </article>
    </section>
  `;
}

function renderBlock(block) {
  switch (block.kind) {
    case "heading":
      return `<h${block.level}>${renderInline(block.text)}</h${block.level}>`;
    case "paragraph":
      return `<p>${renderInline(block.text)}</p>`;
    case "list":
      return `<ul>${block.items.map((item) => `<li>${renderInline(item)}</li>`).join("")}</ul>`;
    case "code":
      return renderCodeBlock(block.code, block.lang);
    case "snippet":
      return renderSnippetCard(block);
    default:
      return "";
  }
}

function renderCodeBlock(code, lang) {
  const langClass = lang ? ` class="language-${escapeAttr(lang)}"` : "";
  return `
    <pre class="code-block"><code${langClass}>${escapeHtml(code)}</code></pre>
  `;
}

function renderSnippetCard(block) {
  const entry = getManifestEntry(block.id);
  const isSelected = state.selectedSnippetId === block.id;
  const modeLabel = entry ? entry.mode : "manifest-missing";
  const docLabel = entry?.docPath || "Manifest doc unavailable";
  const footerLeft = entry ? `Fixture ${entry.fixturePath || "unavailable"}` : "Manifest entry missing";
  const footerRight = entry ? `Mode ${entry.mode}` : "Actions disabled";
  const snippetStatus = entry?.docPath ? `Bound to ${entry.docPath}` : "Unlisted snippet";

  return `
    <button
      type="button"
      class="snippet-card${isSelected ? " is-selected" : ""}"
      data-snippet-id="${escapeAttr(block.id)}"
      aria-pressed="${isSelected ? "true" : "false"}"
    >
      <div class="snippet-card__header">
        <span class="badge">${escapeHtml(modeLabel)}</span>
        <span class="snippet-card__meta">${escapeHtml(block.id)}</span>
      </div>
      <pre><code${block.lang ? ` class="language-${escapeAttr(block.lang)}"` : ""}>${escapeHtml(block.code)}</code></pre>
      <div class="snippet-card__footer">
        <span>${escapeHtml(footerLeft)}</span>
        <span>${escapeHtml(docLabel)}</span>
      </div>
      <div class="snippet-card__footer">
        <span>${escapeHtml(snippetStatus)}</span>
        <span>${escapeHtml(footerRight)}</span>
      </div>
    </button>
  `;
}

function renderPane() {
  const notices = [];

  if (state.docError) {
    notices.push({
      kind: "danger",
      title: "Document load failed",
      copy: state.docError,
    });
  }

  if (state.manifestError) {
    notices.push({
      kind: "warning",
      title: "Manifest unavailable",
      copy: state.manifestError,
    });
  }

  for (const warning of state.manifest?.warnings || []) {
    notices.push(warning);
  }

  const snippet = state.selectedSnippetId ? findSnippetRecord(state.selectedSnippetId) : null;
  const entry = snippet ? getManifestEntry(snippet.id) : null;
  const allowedActions = entry ? actionsForMode(entry.mode) : [];
  const activeAction = allowedActions.includes(state.selectedAction) ? state.selectedAction : defaultActionForMode(entry?.mode) || allowedActions[0] || null;

  if (snippet && !entry) {
    notices.push({
      kind: "warning",
      title: "Manifest entry missing",
      copy: `Snippet ${snippet.id} is rendered, but it has no matching entry in ./examples/manifest.json.`,
    });
  }

  if (entry?.stdoutFixtureWarning) {
    notices.push({
      kind: "warning",
      title: "Fixture fetch warning",
      copy: entry.stdoutFixtureWarning,
    });
  }

  for (const warning of entry?.warnings || []) {
    notices.push({
      kind: "warning",
      title: "Manifest warning",
      copy: warning,
    });
  }

  const resultHtml = snippet
    ? renderActionResult(snippet, entry, activeAction, allowedActions)
    : renderOverviewResult();

  const actionsHtml = renderActionButtons(entry, activeAction, allowedActions);

  els.playgroundPane.innerHTML = `
    <div class="pane-stack">
      <section class="pane-card">
        <p class="pane-kicker">Interaction pane</p>
        <h2 class="pane-title">${snippet ? escapeHtml(snippet.id) : "Overview"}</h2>
        <dl class="pane-meta">
          <div class="pane-meta-row">
            <dt>Route</dt>
            <dd>${escapeHtml(state.route)}</dd>
          </div>
          <div class="pane-meta-row">
            <dt>Document</dt>
            <dd>${escapeHtml(ROUTES[state.route])}</dd>
          </div>
          <div class="pane-meta-row">
            <dt>Selection</dt>
            <dd>${escapeHtml(snippet ? snippet.id : "none")}</dd>
          </div>
          <div class="pane-meta-row">
            <dt>Mode</dt>
            <dd>${escapeHtml(entry?.mode || "unavailable")}</dd>
          </div>
        </dl>
        ${actionsHtml}
      </section>
      ${notices.length ? `<section class="notice-list">${notices.map(renderNotice).join("")}</section>` : ""}
      ${resultHtml}
    </div>
  `;
}

function renderOverviewResult() {
  return `
    <section class="result-card">
      <h3 class="result-card__title">Snippet overview</h3>
      <p class="result-card__copy">Select a snippet card to inspect its manifest metadata and the stubbed Run, Check, Build, or Test result.</p>
      <p class="result-card__copy">The shell stays readable even if the manifest is missing or malformed.</p>
    </section>
  `;
}

function renderActionButtons(entry, activeAction, allowedActions) {
  const disabledAll = !entry || Boolean(state.manifestError);
  const allowed = new Set(allowedActions);

  return `
    <div class="action-grid" role="toolbar" aria-label="Snippet actions">
      ${ACTIONS.map((action) => {
        const isActive = activeAction === action;
        const enabled = !disabledAll && allowed.has(action);
        return `
          <button
            type="button"
            class="action-button"
            data-action="${action}"
            ${isActive ? 'aria-pressed="true"' : 'aria-pressed="false"'}
            ${enabled ? "" : "disabled"}
          >
            ${escapeHtml(actionLabel(action))}
          </button>
        `;
      }).join("")}
    </div>
  `;
}

function renderActionResult(snippet, entry, action, allowedActions) {
  if (!entry) {
    return `
      <section class="result-card result-card--warning">
        <h3 class="result-card__title">No manifest entry</h3>
        <p class="result-card__copy">The snippet renders, but there is no manifest record to derive stubbed actions from.</p>
      </section>
    `;
  }

  if (!action || !allowedActions.includes(action)) {
    return `
      <section class="result-card result-card--warning">
        <h3 class="result-card__title">No active action</h3>
        <p class="result-card__copy">Choose one of the supported actions for this snippet.</p>
      </section>
    `;
  }

  if (action === "run") {
    if (entry.mode !== "run") {
      return unsupportedActionCard(entry, action);
    }

    if (!entry.stdoutFixtureText) {
      return `
        <section class="result-card result-card--warning">
          <h3 class="result-card__title">Run output unavailable</h3>
          <p class="result-card__copy">The manifest requested a stdout fixture, but the browser could not load it.</p>
        </section>
      `;
    }

    return `
      <section class="result-card result-card--success">
        <h3 class="result-card__title">Run output</h3>
        <p class="result-card__copy">Rendered from the manifest's stdout fixture.</p>
        <pre>${escapeHtml(entry.stdoutFixtureText)}</pre>
      </section>
    `;
  }

  if (action === "check") {
    if (entry.mode === "check_ok") {
      return `
        <section class="result-card result-card--success">
          <h3 class="result-card__title">Check passed</h3>
          <p class="result-card__copy">The snippet typechecks under the current compiler contract.</p>
        </section>
      `;
    }

    if (entry.mode === "check_fail") {
      return `
        <section class="result-card result-card--danger">
          <h3 class="result-card__title">Check failed</h3>
          <p class="result-card__copy">Structured diagnostic: <code>${escapeHtml(entry.error_code || "unknown")}</code></p>
        </section>
      `;
    }

    return unsupportedActionCard(entry, action);
  }

  if (action === "build") {
    if (/^build_.*_ok$/.test(entry.mode)) {
      const target = entry.mode.includes("js") ? "JS" : "WASI";
      return `
        <section class="result-card result-card--success">
          <h3 class="result-card__title">Build passed</h3>
          <p class="result-card__copy">${target} emission is supported for this snippet.</p>
        </section>
      `;
    }

    if (/^build_.*_fail$/.test(entry.mode)) {
      return `
        <section class="result-card result-card--danger">
          <h3 class="result-card__title">Build failed</h3>
          <p class="result-card__copy">Expected failure substring: <code>${escapeHtml(entry.error_substring || "unknown")}</code></p>
        </section>
      `;
    }

    return unsupportedActionCard(entry, action);
  }

  if (action === "test") {
    if (entry.mode === "test_ok") {
      return `
        <section class="result-card result-card--success">
          <h3 class="result-card__title">Test passed</h3>
          <p class="result-card__copy">Inline tests execute successfully for this snippet.</p>
        </section>
      `;
    }

    return unsupportedActionCard(entry, action);
  }

  return unsupportedActionCard(entry, action);
}

function unsupportedActionCard(entry, action) {
  return `
    <section class="result-card result-card--warning">
      <h3 class="result-card__title">Action unavailable</h3>
      <p class="result-card__copy">${escapeHtml(actionLabel(action))} is not meaningful for <code>${escapeHtml(entry.mode || "unknown")}</code>.</p>
    </section>
  `;
}

function renderNotice(notice) {
  return `
    <article class="notice notice--${notice.kind}">
      <h3 class="notice__title">${escapeHtml(notice.title)}</h3>
      <p class="notice__copy">${escapeHtml(notice.copy)}</p>
    </article>
  `;
}

function handleDocClick(event) {
  const target = event.target instanceof Element ? event.target.closest("[data-snippet-id]") : null;
  if (!target) {
    return;
  }

  const snippetId = target.getAttribute("data-snippet-id");
  if (!snippetId) {
    return;
  }

  state.selectedSnippetId = snippetId;
  const entry = getManifestEntry(snippetId);
  state.selectedAction = defaultActionForMode(entry?.mode);
  renderMain();
  renderPane();
}

function handlePaneClick(event) {
  const target = event.target instanceof Element ? event.target.closest("[data-action]") : null;
  if (!target || target.hasAttribute("disabled")) {
    return;
  }

  const action = target.getAttribute("data-action");
  const snippetId = state.selectedSnippetId;
  if (!action || !snippetId) {
    return;
  }

  const entry = getManifestEntry(snippetId);
  if (!entry || !actionsForMode(entry.mode).includes(action)) {
    return;
  }

  state.selectedAction = action;
  renderPane();
}

function getManifestEntry(snippetId) {
  return state.manifest?.byId.get(snippetId) || null;
}

function findSnippetRecord(snippetId) {
  if (!state.doc) {
    return null;
  }

  return state.doc.snippetsById.get(snippetId) || null;
}

function defaultActionForMode(mode) {
  if (!mode) {
    return null;
  }

  if (mode === "run") {
    return "run";
  }

  if (mode.startsWith("check_")) {
    return "check";
  }

  if (mode.startsWith("build_")) {
    return "build";
  }

  if (mode === "test_ok") {
    return "test";
  }

  return null;
}

function actionsForMode(mode) {
  if (!mode) {
    return [];
  }

  if (mode === "run") {
    return ["run"];
  }

  if (mode === "check_ok" || mode === "check_fail") {
    return ["check"];
  }

  if (mode.startsWith("build_")) {
    return ["build"];
  }

  if (mode === "test_ok") {
    return ["test"];
  }

  return [];
}

function actionLabel(action) {
  switch (action) {
    case "run":
      return "Run";
    case "check":
      return "Check";
    case "build":
      return "Build";
    case "test":
      return "Test";
    default:
      return action;
  }
}

async function loadDocument(route) {
  if (cache.docs.has(route)) {
    return cache.docs.get(route);
  }

  const promise = (async () => {
    const source = await fetchText(ROUTES[route]);
    return parseMarkdown(source, route);
  })();

  cache.docs.set(route, promise);
  promise.catch(() => {
    cache.docs.delete(route);
  });
  return promise;
}

async function loadManifest() {
  if (cache.manifestPromise) {
    return cache.manifestPromise;
  }

  const promise = (async () => {
    const source = await fetchText("./examples/manifest.json");
    const raw = JSON.parse(source);
    if (!Array.isArray(raw)) {
      throw new Error("Manifest JSON must be an array.");
    }

    const indexed = buildManifestIndex(raw);
    const entries = indexed.entries;

    await Promise.all(
      entries.map(async (entry) => {
        if (!entry.stdoutFixturePath) {
          return;
        }

        try {
          entry.stdoutFixtureText = await fetchText(entry.stdoutFixturePath);
        } catch (error) {
          entry.stdoutFixtureWarning = formatError(
            error,
            `Could not load stdout fixture at ${entry.stdoutFixturePath}.`
          );
        }
      })
    );

    return { ...indexed, warnings: indexed.warnings.map(manifestWarningToNotice) };
  })();

  cache.manifestPromise = promise;
  promise.catch(() => {
    cache.manifestPromise = null;
  });
  return promise;
}

function normalizeManifestEntry(raw, index) {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
    return null;
  }

  const id = typeof raw.id === "string" ? raw.id.trim() : "";
  const mode = typeof raw.mode === "string" ? raw.mode.trim() : "";
  const docPath = normalizeBrowserAssetPath(raw.doc);
  const fixturePath = normalizeBrowserAssetPath(raw.fixture);
  const stdoutFixturePath = normalizeBrowserAssetPath(raw.stdout_fixture);

  if (!id || !mode) {
    return null;
  }

  const warnings = [];
  if (!docPath) {
    warnings.push(`Manifest entry "${id}" has invalid browser asset paths for doc.`);
  }
  if (!fixturePath) {
    warnings.push(`Manifest entry "${id}" has invalid browser asset paths for fixture.`);
  }
  if (raw.stdout_fixture != null && !stdoutFixturePath) {
    warnings.push(`Manifest entry "${id}" has invalid browser asset paths for stdout fixture.`);
  }

  const entry = {
    ...raw,
    id,
    mode,
    docPath,
    fixturePath,
    stdoutFixturePath,
    warnings,
    index,
  };

  return entry;
}

function buildManifestIndex(rawEntries) {
  const warnings = [];
  const entries = [];
  const byId = new Map();

  if (!Array.isArray(rawEntries)) {
    warnings.push("Manifest JSON was not an array.");
    return { entries, byId, warnings };
  }

  for (const [index, item] of rawEntries.entries()) {
    const normalized = normalizeManifestEntry(item, index);
    if (!normalized) {
      warnings.push(
        `Manifest entry at index ${index} is missing a string id, doc path, fixture path, or mode.`
      );
      continue;
    }

    if (byId.has(normalized.id)) {
      warnings.push(
        `Duplicate manifest id "${normalized.id}" at index ${index}; the first entry wins.`
      );
      continue;
    }

    byId.set(normalized.id, normalized);
    entries.push(normalized);
    warnings.push(...normalized.warnings);
  }

  return { entries, byId, warnings };
}

function manifestWarningToNotice(copy) {
  return {
    kind: "warning",
    title: "Manifest warning",
    copy,
  };
}

function normalizeBrowserAssetPath(path) {
  if (typeof path !== "string") {
    return null;
  }

  const trimmed = path.trim();
  if (!trimmed) {
    return null;
  }

  if (/^(?:https?:)?\/\//i.test(trimmed) || trimmed.startsWith("/") || trimmed.startsWith("../")) {
    return null;
  }

  if (trimmed.startsWith("./")) {
    return trimmed;
  }

  const stripped = trimmed.startsWith("docs/") ? trimmed.slice(5) : trimmed;
  if (!stripped || stripped.startsWith("../") || stripped.includes("/../") || stripped.endsWith("/..")) {
    return null;
  }

  return stripped.startsWith("./") || stripped.startsWith("../") ? stripped : `./${stripped}`;
}

async function fetchText(path) {
  if (!path) {
    throw new Error("Missing asset path.");
  }

  const response = await fetch(path);
  if (!response.ok) {
    throw new Error(`Failed to load ${path} (${response.status})`);
  }

  return response.text();
}

function parseMarkdown(source, route) {
  const lines = String(source || "").replace(/\r\n?/g, "\n").split("\n");
  const blocks = [];
  const snippetsById = new Map();

  let title = ROUTE_META[route]?.label || route;
  let summary = "";
  let paragraph = [];
  let listItems = [];
  let pendingSnippetId = null;
  let sawTitle = false;

  const flushParagraph = () => {
    if (!paragraph.length) {
      return;
    }

    const text = paragraph.join(" ").trim();
    paragraph = [];

    if (!text) {
      return;
    }

    blocks.push({ kind: "paragraph", text });
    if (!summary) {
      summary = text;
    }
  };

  const flushList = () => {
    if (!listItems.length) {
      return;
    }

    blocks.push({ kind: "list", items: listItems.slice() });
    listItems = [];
  };

  for (let i = 0; i < lines.length; ) {
    const line = lines[i];
    const trimmed = line.trim();
    const marker = trimmed.match(/^<!--\s*snippet:\s*([A-Za-z0-9_-]+)\s*-->$/);

    if (marker) {
      flushParagraph();
      flushList();
      pendingSnippetId = marker[1];
      i += 1;
      continue;
    }

    if (trimmed === "") {
      if (pendingSnippetId) {
        pendingSnippetId = null;
      }
      flushParagraph();
      flushList();
      i += 1;
      continue;
    }

    if (pendingSnippetId && !trimmed.startsWith("```")) {
      pendingSnippetId = null;
    }

    const heading = trimmed.match(/^(#{1,3})\s+(.*)$/);
    if (heading) {
      flushParagraph();
      flushList();
      const level = heading[1].length;
      const text = heading[2].trim();

      if (level === 1 && !sawTitle) {
        title = text;
        sawTitle = true;
      } else {
        blocks.push({ kind: "heading", level, text });
      }

      i += 1;
      continue;
    }

    const fence = trimmed.match(/^(`{3,})(.*)$/);
    if (fence) {
      flushParagraph();
      flushList();

      const fenceChars = fence[1];
      const lang = fence[2].trim();
      const codeLines = [];
      i += 1;

      while (i < lines.length && !lines[i].trim().startsWith(fenceChars)) {
        codeLines.push(lines[i]);
        i += 1;
      }

      if (i < lines.length) {
        i += 1;
      }

      const code = codeLines.join("\n");
      if (pendingSnippetId) {
        const snippetId = pendingSnippetId;
        pendingSnippetId = null;
        const snippetBlock = { kind: "snippet", id: snippetId, lang, code };
        blocks.push(snippetBlock);
        snippetsById.set(snippetId, snippetBlock);
      } else {
        blocks.push({ kind: "code", lang, code });
      }

      continue;
    }

    if (trimmed.startsWith("- ")) {
      flushParagraph();
      if (!listItems.length) {
        listItems = [];
      }

      while (i < lines.length && lines[i].trim().startsWith("- ")) {
        listItems.push(lines[i].trim().slice(2).trim());
        i += 1;
      }

      flushList();
      continue;
    }

    paragraph.push(trimmed);
    i += 1;
  }

  flushParagraph();
  flushList();

  return {
    title,
    summary,
    blocks,
    snippetsById,
  };
}

function renderInline(text) {
  const parts = String(text).split("`");
  return parts
    .map((part, index) => (index % 2 === 1 ? `<code>${escapeHtml(part)}</code>` : escapeHtml(part)))
    .join("");
}

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function escapeAttr(value) {
  return escapeHtml(value).replace(/`/g, "&#96;");
}

function formatError(error, fallbackMessage) {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  if (typeof error === "string" && error.trim()) {
    return error;
  }

  return fallbackMessage;
}

if (typeof exports !== "undefined") {
  exports.resolveRouteFromHash = resolveRouteFromHash;
  exports.normalizeBrowserAssetPath = normalizeBrowserAssetPath;
  exports.parseMarkdown = parseMarkdown;
  exports.buildManifestIndex = buildManifestIndex;
}
