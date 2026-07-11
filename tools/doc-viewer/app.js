/* Arukellt doc viewer — self-contained SPA logic.
 *
 * Routes:
 *   #/docs/<path-without-.md>  ->  fetch /docs/<path>.md  and render
 *   #/                          ->  /docs/README.md
 *
 * Sidebar: built from /api/tree (server-side directory listing of docs/).
 *   Directories are expandable/collapsible. Files (.md, .html) are links.
 *   The active file is highlighted and its parent dirs auto-expanded.
 *
 * Search: lazily builds an in-memory index of all .md files from the tree.
 *   Plain substring match on raw markdown text.
 */
(function () {
  "use strict";

  const DOCS_PREFIX = "/docs/";
  const TREE_URL = "/api/tree";
  const HOME_ROUTE = "/docs/README";

  const contentEl = document.getElementById("content");
  const sidebarNav = document.getElementById("sidebar-nav");
  const searchInput = document.getElementById("search-input");
  const searchResults = document.getElementById("search-results");
  const sidebarEl = document.getElementById("sidebar");
  const menuToggle = document.getElementById("menu-toggle");

  // Configure marked: GFM tables.
  marked.setOptions({ gfm: true, breaks: false });

  // ---------- route helpers ----------
  function currentRoute() {
    const h = window.location.hash || "#/";
    if (h === "#/" || h === "#") return HOME_ROUTE;
    return h.slice(2); // strip "#/"
  }

  function routeToUrl(route) {
    if (route.endsWith(".md")) return route;
    if (route.endsWith(".html")) return route;
    return route + ".md";
  }

  // ---------- state ----------
  let treeData = null;
  // all doc routes discovered from tree, for search
  const knownRoutes = new Set();
  // raw markdown cache: route -> text
  const mdCache = new Map();
  // collapsed dirs: Set of dir paths
  const collapsedDirs = new Set();

  async function fetchText(url) {
    const res = await fetch(url);
    if (!res.ok) throw new Error("HTTP " + res.status + " for " + url);
    return res.text();
  }

  async function fetchJson(url) {
    const res = await fetch(url);
    if (!res.ok) throw new Error("HTTP " + res.status + " for " + url);
    return res.json();
  }

  // ---------- tree ----------
  function collectRoutes(nodes) {
    nodes.forEach((n) => {
      if (n.type === "file") {
        const route = "/docs/" + n.path.replace(/\.md$/, "");
        knownRoutes.add(route);
      }
      if (n.type === "dir" && n.children) {
        collectRoutes(n.children);
      }
    });
  }

  function routeToFilePath(route) {
    // route like "/docs/compiler/README" -> "compiler/README.md"
    return route.replace(/^\/docs\//, "") + ".md";
  }

  function filePathToRoute(filePath) {
    return "/docs/" + filePath.replace(/\.md$/, "");
  }

  function isAncestorDir(dirPath, filePath) {
    // dirPath like "compiler", filePath like "compiler/README.md"
    return filePath.startsWith(dirPath + "/");
  }

  function buildTreeHtml(nodes, level) {
    const ul = document.createElement("ul");
    ul.className = "tree-level-" + level;
    nodes.forEach((n) => {
      const li = document.createElement("li");
      li.className = "tree-node";
      if (n.type === "dir") {
        const hasMdChild = hasMdInTree(n.children);
        if (!hasMdChild) return; // skip dirs with no .md/.html

        const row = document.createElement("div");
        row.className = "tree-row tree-dir-row";
        const chevron = document.createElement("span");
        chevron.className = "tree-chevron";
        chevron.textContent = "▶";
        const label = document.createElement("span");
        label.className = "tree-dir-label";
        label.textContent = n.name;
        row.appendChild(chevron);
        row.appendChild(label);
        li.appendChild(row);

        const childUl = buildTreeHtml(n.children, level + 1);
        childUl.style.display = "none"; // collapsed by default
        li.appendChild(childUl);

        row.addEventListener("click", () => {
          const expanded = childUl.style.display !== "none";
          childUl.style.display = expanded ? "none" : "";
          chevron.classList.toggle("expanded", !expanded);
          if (!expanded) {
            collapsedDirs.delete(n.path);
          } else {
            collapsedDirs.add(n.path);
          }
        });
      } else {
        // file
        const route = filePathToRoute(n.path);
        const a = document.createElement("a");
        a.className = "tree-row tree-file-row";
        a.href = "#/" + route;
        a.setAttribute("data-route", route);
        const icon = document.createElement("span");
        icon.className = "tree-file-icon";
        icon.textContent = "📄";
        const label = document.createElement("span");
        label.className = "tree-file-label";
        // strip .md extension for display
        label.textContent = n.name.replace(/\.md$/, "");
        a.appendChild(icon);
        a.appendChild(label);
        li.appendChild(a);
      }
      ul.appendChild(li);
    });
    return ul;
  }

  function hasMdInTree(nodes) {
    if (!nodes) return false;
    for (const n of nodes) {
      if (n.type === "file") return true;
      if (n.type === "dir" && hasMdInTree(n.children)) return true;
    }
    return false;
  }

  function autoExpandForRoute(route) {
    // route like "/docs/compiler/README" -> filePath "compiler/README.md"
    const filePath = routeToFilePath(route);
    // walk tree, expand all ancestor dirs
    function walk(nodes) {
      let found = false;
      nodes.forEach((n) => {
        if (n.type === "dir") {
          if (isAncestorDir(n.path, filePath) || n.path + "/" + "README.md" === filePath) {
            found = true;
          }
          if (n.children && walk(n.children)) {
            // expand this dir
            found = true;
          }
        }
        if (n.type === "file" && n.path === filePath) {
          found = true;
        }
      });
      return found;
    }
    // Actually simpler: just expand all dirs that are prefixes of filePath
    function expandAncestors(nodes) {
      nodes.forEach((n) => {
        if (n.type === "dir") {
          if (isAncestorDir(n.path, filePath)) {
            collapsedDirs.delete(n.path);
          }
          if (n.children) expandAncestors(n.children);
        }
      });
    }
    expandAncestors(treeData);
  }

  function applyCollapseState(ul) {
    // After building or rebuilding, hide collapsed dirs
    ul.querySelectorAll(".tree-dir-row").forEach((row) => {
      const li = row.parentElement;
      const childUl = li.querySelector(":scope > ul");
      if (!childUl) return;
      // find dir path from the label
      const label = row.querySelector(".tree-dir-label").textContent;
      // we need the path — store it on the row
    });
  }

  function renderSidebarTree() {
    if (!treeData) return;
    sidebarNav.innerHTML = "";
    const tree = buildTreeHtml(treeData, 0);
    sidebarNav.appendChild(tree);
  }

  function highlightActive(route) {
    const target = "#/" + route;
    sidebarNav.querySelectorAll(".tree-file-row").forEach((a) => {
      const r = a.getAttribute("data-route");
      a.classList.toggle("active", r === route);
    });
  }

  function expandAncestorsInDom(route) {
    // Walk DOM tree, expand all dirs that are ancestors of the active file
    const filePath = routeToFilePath(route);
    function walk(ul) {
      ul.querySelectorAll(":scope > li").forEach((li) => {
        const row = li.querySelector(":scope > .tree-dir-row");
        if (!row) return;
        const dirPath = row.getAttribute("data-path");
        if (!dirPath) return;
        const childUl = li.querySelector(":scope > ul");
        if (!childUl) return;
        if (isAncestorDir(dirPath, filePath)) {
          childUl.style.display = "";
          const chev = row.querySelector(".tree-chevron");
          if (chev) chev.classList.add("expanded");
        } else if (!collapsedDirs.has(dirPath)) {
          // not an ancestor — respect collapsed state (default collapsed)
        }
        walk(childUl);
      });
    }
    walk(sidebarNav);
  }

  async function loadTree() {
    if (treeData) return;
    try {
      treeData = await fetchJson(TREE_URL);
      collectRoutes(treeData);
      renderSidebarTree();
      // store dir paths on rows for later lookup
      storeDirPaths(sidebarNav, treeData, "");
    } catch (e) {
      sidebarNav.innerHTML =
        '<div style="padding:14px 18px;color:#b00020">ツリー読み込み失敗: ' +
        e.message + "</div>";
    }
  }

  function storeDirPaths(root, nodes, parentPath) {
    // Attach data-path to each dir row so expandAncestorsInDom can find them
    const rows = root.querySelectorAll(":scope > li > .tree-dir-row");
    let i = 0;
    nodes.forEach((n) => {
      if (n.type === "dir" && hasMdInTree(n.children)) {
        const row = rows[i];
        if (row) {
          row.setAttribute("data-path", n.path);
          const childUl = row.parentElement.querySelector(":scope > ul");
          if (childUl && n.children) {
            storeDirPaths(childUl, n.children, n.path);
          }
        }
        i++;
      }
    });
  }

  // ---------- heading ids ----------
  function slugify(text) {
    return text.toLowerCase()
      .replace(/[^\w\s-]/g, "")
      .replace(/\s+/g, "-")
      .replace(/-+/g, "-")
      .replace(/^-|-$/g, "");
  }

  function addHeadingIds(root) {
    const counts = {};
    let idx = 0;
    root.querySelectorAll("h1, h2, h3, h4, h5, h6").forEach((h) => {
      const text = h.textContent.trim();
      let slug = slugify(text);
      if (!slug) slug = "heading-" + idx;
      idx++;
      if (counts[slug] !== undefined) {
        counts[slug]++;
        slug = slug + "-" + counts[slug];
      } else {
        counts[slug] = 0;
      }
      h.id = slug;
    });
  }

  // ---------- content rendering ----------
  function rewriteContentLinks(root, baseRoute) {
    const baseDir = baseRoute.replace(/\/[^/]*$/, "/");
    root.querySelectorAll("a[href]").forEach((a) => {
      const href = a.getAttribute("href") || "";
      if (/^[a-z][a-z0-9+.-]*:/i.test(href)) return; // external
      if (href.startsWith("#") && !href.startsWith("#/")) {
        // in-page anchor
        a.addEventListener("click", (e) => {
          const id = href.slice(1);
          const target = document.getElementById(id);
          if (target) {
            e.preventDefault();
            target.scrollIntoView({ behavior: "smooth", block: "start" });
          }
        });
        return;
      }
      if (href.startsWith("#/")) {
        const tail = href.slice(2);
        a.setAttribute("href", "#/docs/" + tail);
        knownRoutes.add("/docs/" + tail.replace(/\.md$/, ""));
        return;
      }
      if (/\.md($|[?#])/i.test(href)) {
        const clean = href.replace(/\.md($|[?#].*)/, "$1");
        const resolved = new URL(clean, "http://x/" + baseDir).pathname.replace(/^\//, "/");
        a.setAttribute("href", "#/" + resolved);
        knownRoutes.add(resolved);
      }
    });
  }

  async function renderRoute(route) {
    contentEl.classList.add("loading");
    contentEl.classList.remove("error");
    contentEl.innerHTML = "";

    const url = routeToUrl(route);
    let text;
    try {
      if (mdCache.has(route)) {
        text = mdCache.get(route);
      } else {
        text = await fetchText(url);
        mdCache.set(route, text);
      }
    } catch (e) {
      contentEl.classList.remove("loading");
      contentEl.classList.add("error");
      contentEl.textContent = "読み込み失敗: " + e.message;
      return;
    }

    const html = marked.parse(text);
    contentEl.classList.remove("loading");
    contentEl.innerHTML = html;
    addHeadingIds(contentEl);
    rewriteContentLinks(contentEl, route);
    highlightActive(route);
    expandAncestorsInDom(route);

    // breadcrumb
    const bc = document.createElement("div");
    bc.className = "breadcrumb";
    const parts = route.replace(/^\/docs\//, "").split("/");
    let acc = "/docs";
    bc.innerHTML = '<a href="#/docs/README">docs</a>';
    parts.forEach((p, i) => {
      acc += "/" + p;
      if (i === parts.length - 1) {
        bc.insertAdjacentHTML("beforeend", " / " + p);
      } else {
        bc.insertAdjacentHTML("beforeend", ' / <a href="#' + acc + '">' + p + "</a>");
      }
    });
    contentEl.insertBefore(bc, contentEl.firstChild);

    document.querySelector(".main").scrollTop = 0;
    sidebarEl.classList.remove("open");
    knownRoutes.add(route);
  }

  // ---------- search ----------
  let searchIndexBuilt = false;
  let buildingIndex = false;

  async function buildSearchIndex() {
    if (searchIndexBuilt || buildingIndex) return;
    buildingIndex = true;
    const queue = Array.from(knownRoutes);
    const seen = new Set();
    while (queue.length) {
      const r = queue.shift();
      if (seen.has(r)) continue;
      seen.add(r);
      const url = routeToUrl(r);
      try {
        if (!mdCache.has(r)) {
          const text = await fetchText(url);
          mdCache.set(r, text);
        }
        const linkRe = /\]\(([^)]+\.md)\)/g;
        let m;
        const baseDir = r.replace(/\/[^/]*$/, "/");
        while ((m = linkRe.exec(mdCache.get(r))) !== null) {
          const raw = m[1].split(/[#?]/)[0];
          if (/^https?:/i.test(raw)) continue;
          const resolved = new URL(raw, "http://x/" + baseDir).pathname.replace(/^\//, "/");
          const route = "/docs/" + resolved.replace(/^docs\//, "").replace(/\.md$/, "");
          if (!seen.has(route)) queue.push(route);
        }
      } catch (_) {
        // skip missing
      }
    }
    searchIndexBuilt = true;
    buildingIndex = false;
  }

  function titleOf(route, text) {
    const m = text.match(/^#\s+(.+)$/m);
    if (m) return m[1].trim();
    return route.replace(/^\/docs\//, "");
  }

  function renderSearchResults(query) {
    searchResults.innerHTML = "";
    if (!query) {
      searchResults.style.display = "none";
      return;
    }
    searchResults.style.display = "block";
    const q = query.toLowerCase();
    const hits = [];
    mdCache.forEach((text, route) => {
      const lower = text.toLowerCase();
      let idx = lower.indexOf(q);
      if (idx < 0) return;
      const start = Math.max(0, idx - 40);
      const ctx = text.slice(start, idx + query.length + 60).replace(/\s+/g, " ").trim();
      hits.push({ route, title: titleOf(route, text), ctx });
    });
    hits.sort((a, b) => a.title.localeCompare(b.title));
    if (hits.length === 0) {
      searchResults.innerHTML = '<div class="group">見つかりません</div>';
      return;
    }
    hits.slice(0, 60).forEach((h) => {
      const el = document.createElement("a");
      el.className = "hit";
      el.href = "#/" + h.route;
      el.innerHTML = escapeHtml(h.title) + ' <span class="ctx">' + escapeHtml(h.ctx) + "</span>";
      searchResults.appendChild(el);
    });
  }

  function escapeHtml(s) {
    return s.replace(/[&<>"']/g, (c) => ({
      "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
    })[c]);
  }

  // ---------- events ----------
  let searchTimer = null;
  searchInput.addEventListener("input", () => {
    clearTimeout(searchTimer);
    const q = searchInput.value.trim();
    if (!q) {
      searchResults.style.display = "none";
      return;
    }
    searchTimer = setTimeout(async () => {
      searchResults.innerHTML = '<div class="group">インデックス構築中…</div>';
      searchResults.style.display = "block";
      await buildSearchIndex();
      renderSearchResults(q);
    }, 180);
  });

  searchInput.addEventListener("focus", () => {
    if (searchInput.value.trim()) {
      searchResults.style.display = "block";
    }
  });

  document.addEventListener("click", (e) => {
    if (!e.target.closest(".sidebar")) {
      searchResults.style.display = "none";
    }
  });

  menuToggle.addEventListener("click", () => {
    sidebarEl.classList.toggle("open");
  });

  function onHashChange() {
    const route = currentRoute();
    renderRoute(route);
  }
  window.addEventListener("hashchange", onHashChange);

  // ---------- boot ----------
  (async function boot() {
    await loadTree();
    const route = currentRoute();
    renderRoute(route);
  })();
})();
