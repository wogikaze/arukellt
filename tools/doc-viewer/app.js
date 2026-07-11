/* Arukellt doc viewer — self-contained SPA logic.
 *
 * Routes:
 *   #/docs/<path-without-.md>  ->  fetch /docs/<path>.md  and render
 *   #/                          ->  /docs/README.md
 *
 * Sidebar: built from /docs/_sidebar.md (docsify-compatible link format
 *   like #/README, #/compiler/README) — rewritten to #/docs/<...>.
 *
 * Search: lazily builds an in-memory index of all .md files referenced
 *   from the sidebar (plus their sub-links discovered by scanning the
 *   rendered content). Plain substring match on raw markdown text.
 */
(function () {
  "use strict";

  const DOCS_PREFIX = "/docs/";
  const SIDEBAR_URL = DOCS_PREFIX + "_sidebar.md";
  const HOME_ROUTE = "/docs/README";

  const contentEl = document.getElementById("content");
  const sidebarNav = document.getElementById("sidebar-nav");
  const searchInput = document.getElementById("search-input");
  const searchResults = document.getElementById("search-results");
  const sidebarEl = document.getElementById("sidebar");
  const menuToggle = document.getElementById("menu-toggle");

  // Configure marked: GFM tables, line breaks.
  marked.setOptions({ gfm: true, breaks: false, headerIds: true, mangle: false });

  // ---------- route helpers ----------
  function currentRoute() {
    const h = window.location.hash || "#/";
    if (h === "#/" || h === "#") return HOME_ROUTE;
    return h.slice(2); // strip "#/"
  }

  function routeToUrl(route) {
    // route like "/docs/compiler/README" -> "/docs/compiler/README.md"
    if (route.endsWith(".md")) return route;
    return route + ".md";
  }

  function navigate(route) {
    if (window.location.hash !== "#/" + route) {
      window.location.hash = "#/" + route;
    } else {
      renderRoute(route);
    }
  }

  // ---------- sidebar ----------
  let sidebarLoaded = false;
  // all doc routes discovered from sidebar + content, for search
  const knownRoutes = new Set();
  // raw markdown cache: route -> text
  const mdCache = new Map();

  async function fetchText(url) {
    const res = await fetch(url);
    if (!res.ok) throw new Error("HTTP " + res.status + " for " + url);
    return res.text();
  }

  function rewriteSidebarLinks(root) {
    // docsify sidebar links: #/README, #/compiler/README, #/overview.html
    // -> #/docs/README, #/docs/compiler/README, #/docs/overview.html
    root.querySelectorAll("a[href]").forEach((a) => {
      const href = a.getAttribute("href") || "";
      if (href.startsWith("#/")) {
        const tail = href.slice(2);
        a.setAttribute("href", "#/docs/" + tail);
        const route = "/docs/" + tail.replace(/\.md$/, "");
        knownRoutes.add(route);
      }
    });
  }

  function buildSidebarTree(sidebarHtml) {
    // _sidebar.md uses bullet groups like:
    //   - **Group**
    //     - [Label](#/path)
    // marked renders these as nested <ul><li><a>. We walk the top-level
    // <ul> and detect group headers (bold text in <strong>).
    const tmp = document.createElement("div");
    tmp.innerHTML = sidebarHtml;
    rewriteSidebarLinks(tmp);

    sidebarNav.innerHTML = "";
    const uls = tmp.querySelectorAll("ul");
    if (uls.length === 0) {
      sidebarNav.innerHTML = '<div style="padding:14px 18px;color:var(--muted)">サイドバーが空です</div>';
      return;
    }

    const topUl = uls[0];
    const out = document.createElement("ul");
    topUl.childNodes.forEach((li) => {
      if (li.nodeType !== 1) return;
      if (li.tagName.toLowerCase() !== "li") return;
      // group header: contains <strong> as direct text
      const strong = li.querySelector(":scope > strong");
      const anchor = li.querySelector(":scope > a");
      if (strong && !anchor) {
        const title = document.createElement("li");
        title.className = "group-title";
        title.textContent = strong.textContent;
        out.appendChild(title);
        // nested ul
        const nested = li.querySelector(":scope > ul");
        if (nested) {
          nested.childNodes.forEach((sub) => {
            if (sub.nodeType !== 1 || sub.tagName.toLowerCase() !== "li") return;
            const a = sub.querySelector("a");
            if (!a) return;
            const wrap = document.createElement("li");
            wrap.className = "indent";
            wrap.appendChild(a.cloneNode(true));
            out.appendChild(wrap);
          });
        }
      } else if (anchor) {
        const wrap = document.createElement("li");
        wrap.appendChild(anchor.cloneNode(true));
        out.appendChild(wrap);
      }
    });
    sidebarNav.appendChild(out);
  }

  async function loadSidebar() {
    if (sidebarLoaded) return;
    try {
      const md = await fetchText(SIDEBAR_URL);
      const html = marked.parse(md);
      buildSidebarTree(html);
      sidebarLoaded = true;
    } catch (e) {
      sidebarNav.innerHTML =
        '<div style="padding:14px 18px;color:#b00020">サイドバー読み込み失敗: ' +
        e.message + "</div>";
    }
  }

  function highlightActive(route) {
    const target = "#/docs/" + route.replace(/^\/docs\//, "").replace(/\.md$/, "");
    sidebarNav.querySelectorAll("a").forEach((a) => {
      const href = a.getAttribute("href") || "";
      a.classList.toggle("active", href === target);
    });
  }

  // ---------- content rendering ----------
  function rewriteContentLinks(root, baseRoute) {
    // Relative .md links inside content: resolve against baseRoute dir.
    // e.g. in /docs/compiler/README, a link to "pipeline.md" -> #/docs/compiler/pipeline
    const baseDir = baseRoute.replace(/\/[^/]*$/, "/");
    root.querySelectorAll("a[href]").forEach((a) => {
      const href = a.getAttribute("href") || "";
      if (/^[a-z][a-z0-9+.-]*:/i.test(href)) return; // external
      if (href.startsWith("#")) return; // in-page anchor
      if (href.startsWith("#/")) {
        // docsify-style absolute hash link
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
    rewriteContentLinks(contentEl, route);
    highlightActive(route);

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

    // scroll to top (or to anchor)
    document.querySelector(".main").scrollTop = 0;
    if (window.location.hash.includes("#") && route.includes("#")) {
      // in-page anchor handled by browser
    }

    // close mobile sidebar on navigation
    sidebarEl.classList.remove("open");

    // register discovered sub-routes for search
    knownRoutes.add(route);
  }

  // ---------- search ----------
  let searchIndexBuilt = false;
  let buildingIndex = false;

  async function buildSearchIndex() {
    if (searchIndexBuilt || buildingIndex) return;
    buildingIndex = true;
    // gather routes: sidebar already populated knownRoutes with top-level.
    // Also scan each loaded doc for relative .md links to discover more.
    const queue = Array.from(knownRoutes);
    const seen = new Set();
    while (queue.length) {
      const r = queue.shift();
      if (seen.has(r)) continue;
      seen.add(r);
      if (!r.endsWith(".md") && !r.endsWith(".html")) {
        // fetch as .md
      }
      const url = r.endsWith(".md") ? DOCS_PREFIX + r.replace(/^\/docs\//, "") : routeToUrl(r);
      try {
        if (!mdCache.has(r)) {
          const text = await fetchText(url);
          mdCache.set(r, text);
        }
        // discover relative .md links in this text
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

  // hash routing
  function onHashChange() {
    const route = currentRoute();
    renderRoute(route);
  }
  window.addEventListener("hashchange", onHashChange);

  // ---------- boot ----------
  (async function boot() {
    await loadSidebar();
    const route = currentRoute();
    if (route === HOME_ROUTE && (window.location.hash === "" || window.location.hash === "#" || window.location.hash === "#/")) {
      // ensure URL reflects home
    }
    renderRoute(route);
  })();
})();
