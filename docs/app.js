const ROUTES = {
  "language-tour": {
    label: "Language Tour",
    docPath: "./language-tour.md",
  },
  std: {
    label: "Std",
    docPath: "./std.md",
  },
};

const ACTION_LABELS = {
  run: "Run",
  check_ok: "Check",
  check_fail: "Check",
  build_js_ok: "Build",
  build_wasi_ok: "Build",
  test_ok: "Test",
};

function escapeHtml(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

export function resolveRouteFromHash(hash) {
  const normalized = (hash ?? "").replace(/^#\/?/, "").trim();
  return Object.hasOwn(ROUTES, normalized) ? normalized : "language-tour";
}

export function normalizeBrowserAssetPath(path) {
  if (typeof path !== "string") {
    return null;
  }
  if (path.startsWith("/") || path.includes("..")) {
    return null;
  }
  if (path === "docs" || path.startsWith("docs/")) {
    return `.${path.slice("docs".length)}`;
  }
  if (path.startsWith("./")) {
    return path;
  }
  return `./${path}`;
}

export function buildManifestIndex(entries) {
  const manifestEntries = [];
  const warnings = [];
  const seenIds = new Set();

  for (const entry of Array.isArray(entries) ? entries : []) {
    if (typeof entry?.id !== "string" || entry.id.length === 0) {
      warnings.push("Manifest entry missing a string id");
      continue;
    }
    if (seenIds.has(entry.id)) {
      warnings.push(`Duplicate manifest id: ${entry.id}`);
      continue;
    }
    seenIds.add(entry.id);

    const normalized = { ...entry };
    const normalizedPaths = {};
    let hasInvalidPath = false;
    for (const key of ["doc", "fixture", "stdout_fixture", "stderr_fixture"]) {
      if (!Object.hasOwn(entry, key)) {
        continue;
      }
      const normalizedPath = normalizeBrowserAssetPath(entry[key]);
      normalizedPaths[key] = normalizedPath;
      hasInvalidPath ||= normalizedPath === null;
    }
    if (hasInvalidPath) {
      warnings.push(`Manifest entry ${entry.id} contains invalid browser asset paths`);
    }

    normalized.browser = normalizedPaths;
    manifestEntries.push(normalized);
  }

  return { entries: manifestEntries, warnings };
}

function parseInline(text) {
  return escapeHtml(text).replace(/`([^`]+)`/g, "<code>$1</code>");
}

function parseFence(lines, start) {
  const fence = lines[start];
  const language = fence.slice(3).trim() || "text";
  const body = [];
  let index = start + 1;
  while (index < lines.length && !lines[index].startsWith("```")) {
    body.push(lines[index]);
    index += 1;
  }
  if (index < lines.length) {
    index += 1;
  }
  return {
    index,
    language,
    code: body.join("\n"),
  };
}

export function parseMarkdown(markdown, route) {
  const lines = markdown.replaceAll("\r\n", "\n").split("\n");
  const blocks = [];
  let index = 0;

  while (index < lines.length) {
    const line = lines[index];
    if (line.trim() === "") {
      index += 1;
      continue;
    }

    const heading = line.match(/^(#{1,6})\s+(.*)$/);
    if (heading) {
      blocks.push({
        kind: "heading",
        level: heading[1].length,
        text: heading[2].trim(),
      });
      index += 1;
      continue;
    }

    const snippet = line.match(/^<!--\s*snippet:\s*([A-Za-z0-9_-]+)\s*-->$/);
    if (snippet) {
      if (lines[index + 1]?.startsWith("```")) {
        const fence = parseFence(lines, index + 1);
        blocks.push({
          kind: "snippet",
          id: snippet[1],
          route,
          language: fence.language,
          code: fence.code,
        });
        index = fence.index;
      } else {
        index += 1;
      }
      continue;
    }

    if (line.startsWith("```")) {
      const fence = parseFence(lines, index);
      blocks.push({
        kind: "code",
        language: fence.language,
        code: fence.code,
      });
      index = fence.index;
      continue;
    }

    const paragraph = [line];
    index += 1;
    while (
      index < lines.length &&
      lines[index].trim() !== "" &&
      !lines[index].startsWith("```") &&
      !lines[index].startsWith("<!-- snippet:") &&
      !/^(#{1,6})\s+/.test(lines[index])
    ) {
      paragraph.push(lines[index]);
      index += 1;
    }
    blocks.push({
      kind: "paragraph",
      text: paragraph.join(" "),
    });
  }

  const html = blocks
    .map((block) => {
      if (block.kind === "heading") {
        return `<h${block.level}>${escapeHtml(block.text)}</h${block.level}>`;
      }
      if (block.kind === "paragraph") {
        return `<p>${parseInline(block.text)}</p>`;
      }
      if (block.kind === "code") {
        return `<pre><code class="language-${escapeHtml(block.language)}">${escapeHtml(block.code)}</code></pre>`;
      }
      return [
        `<article class="doc-snippet" data-snippet-id="${escapeHtml(block.id)}">`,
        `<div class="doc-snippet-meta">Snippet: <code>${escapeHtml(block.id)}</code></div>`,
        `<pre><code class="language-${escapeHtml(block.language)}">${escapeHtml(block.code)}</code></pre>`,
        "</article>",
      ].join("");
    })
    .join("\n");

  return { blocks, html };
}

function ensureShell() {
  let sidebar = document.getElementById("sidebar");
  let docView = document.getElementById("doc-view");
  let playgroundPane = document.getElementById("playground-pane");
  if (sidebar && docView && playgroundPane) {
    return { sidebar, docView, playgroundPane };
  }

  const app = document.getElementById("app");
  if (!app) {
    throw new Error("docs shell missing");
  }

  app.innerHTML = `
    <div class="docs-layout">
      <nav id="sidebar" class="sidebar"></nav>
      <main id="doc-view" class="docs-content"></main>
      <aside id="playground-pane" class="playground-pane"></aside>
    </div>
  `;

  sidebar = document.getElementById("sidebar");
  docView = document.getElementById("doc-view");
  playgroundPane = document.getElementById("playground-pane");
  return { sidebar, docView, playgroundPane };
}

function routeLinks() {
  return Object.entries(ROUTES)
    .map(
      ([id, route]) =>
        `<a class="sidebar-link" href="#/${id}">${escapeHtml(route.label)}</a>`,
    )
    .join("");
}

function renderSidebar(sidebar, manifest, route) {
  const manifestWarnings = manifest.warnings.length
    ? `<div class="manifest-warning">Manifest doc unavailable: ${escapeHtml(manifest.warnings.join(" | "))}</div>`
    : "";
  sidebar.innerHTML = [
    `<div class="sidebar-section"><div class="sidebar-label">Routes</div>${routeLinks()}</div>`,
    `<div class="sidebar-section"><div class="sidebar-label">Selected</div><div>${escapeHtml(
      ROUTES[route].label,
    )}</div></div>`,
    manifestWarnings,
  ].join("");
}

function entriesForRoute(manifest, route) {
  return manifest.entries.filter((entry) => entry.browser.doc === ROUTES[route].docPath);
}

function actionLabel(entry) {
  return ACTION_LABELS[entry.mode] ?? "Run";
}

function renderOverview(entries) {
  const items = entries.length
    ? entries
        .map(
          (entry) =>
            `<li><button type="button" data-snippet-id="${escapeHtml(entry.id)}">${escapeHtml(
              entry.id,
            )}</button></li>`,
        )
        .join("")
    : "<li>No snippets registered for this page.</li>";
  return [
    "<section>",
    "<h2>Snippet overview</h2>",
    "<p>Select a snippet in the document to inspect its fixture and actions.</p>",
    `<ul>${items}</ul>`,
    "</section>",
  ].join("");
}

async function renderSelectedSnippet(playgroundPane, entry, action) {
  const selectedAction = action ?? actionLabel(entry).toLowerCase();
  const buttons = ["run", "check", "build", "test"]
    .map(
      (candidate) =>
        `<button type="button" data-action="${candidate}"${
          candidate === selectedAction ? " data-selected=\"true\"" : ""
        }>${candidate[0].toUpperCase()}${candidate.slice(1)}</button>`,
    )
    .join("");

  const base = [
    "<section>",
    `<h2>${actionLabel(entry)} output</h2>`,
    `<div class="snippet-id"><code>${escapeHtml(entry.id)}</code></div>`,
    `<div class="snippet-actions">${buttons}</div>`,
    "</section>",
  ].join("");

  playgroundPane.innerHTML = `${base}<p>Loading fixture output...</p>`;

  if (!entry.browser.stdout_fixture) {
    playgroundPane.innerHTML = `${base}<p>Fixture has no stdout companion.</p>`;
    return;
  }

  try {
    const stdout = await fetchText(entry.browser.stdout_fixture);
    playgroundPane.innerHTML = `${base}<pre>${escapeHtml(stdout)}</pre>`;
  } catch (error) {
    playgroundPane.innerHTML = `${base}<p>Failed to load fixture output: ${escapeHtml(
      error.message,
    )}</p>`;
  }
}

async function fetchText(path) {
  const response = await fetch(path);
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`);
  }
  return response.text();
}

async function fetchManifest() {
  try {
    const raw = await fetchText("./examples/manifest.json");
    return buildManifestIndex(JSON.parse(raw));
  } catch (error) {
    return {
      entries: [],
      warnings: [`Manifest doc unavailable: ${error.message}`],
    };
  }
}

async function renderRoute(route, selectedSnippetId = null, selectedAction = null) {
  const { sidebar, docView, playgroundPane } = ensureShell();
  const manifest = await fetchManifest();
  renderSidebar(sidebar, manifest, route);

  const markdown = await fetchText(ROUTES[route].docPath);
  const parsed = parseMarkdown(markdown, route);
  docView.innerHTML = parsed.html;

  const routeEntries = entriesForRoute(manifest, route);
  const selected = routeEntries.find((entry) => entry.id === selectedSnippetId) ?? null;
  if (selected) {
    await renderSelectedSnippet(playgroundPane, selected, selectedAction);
  } else {
    playgroundPane.innerHTML = renderOverview(routeEntries);
  }

  return { manifest, routeEntries };
}

async function boot() {
  const route = resolveRouteFromHash(globalThis.location?.hash ?? "");
  if ((globalThis.location?.hash ?? "") === "") {
    history.replaceState(null, "", `#/${route}`);
  }

  let selectedSnippetId = null;
  let selectedAction = null;
  let currentRoute = route;
  let currentRouteEntries = [];
  const shell = ensureShell();

  shell.docView.addEventListener("click", async (event) => {
    const target = event.target instanceof Element ? event.target : null;
    const snippetNode = target?.closest?.("[data-snippet-id]");
    if (!snippetNode) {
      return;
    }
    selectedSnippetId = snippetNode.getAttribute("data-snippet-id");
    selectedAction = "run";
    const entry = currentRouteEntries.find((candidate) => candidate.id === selectedSnippetId);
    if (entry) {
      await renderSelectedSnippet(shell.playgroundPane, entry, selectedAction);
    }
  });

  shell.playgroundPane.addEventListener("click", async (event) => {
    const target = event.target instanceof Element ? event.target : null;
    const snippetNode = target?.closest?.("[data-snippet-id]");
    if (snippetNode) {
      selectedSnippetId = snippetNode.getAttribute("data-snippet-id");
      selectedAction = "run";
    }
    const actionNode = target?.closest?.("[data-action]");
    if (actionNode) {
      selectedAction = actionNode.getAttribute("data-action");
    }
    const entry = currentRouteEntries.find((candidate) => candidate.id === selectedSnippetId);
    if (entry) {
      await renderSelectedSnippet(shell.playgroundPane, entry, selectedAction);
    }
  });

  const initial = await renderRoute(route, selectedSnippetId, selectedAction);
  currentRouteEntries = initial.routeEntries;

  if (typeof window !== "undefined") {
    window.addEventListener("hashchange", async () => {
      currentRoute = resolveRouteFromHash(globalThis.location?.hash ?? "");
      const rendered = await renderRoute(currentRoute, null, null);
      currentRouteEntries = rendered.routeEntries;
      selectedSnippetId = null;
      selectedAction = null;
    });
  }
}

if (typeof document !== "undefined") {
  document.addEventListener("DOMContentLoaded", boot);
}
