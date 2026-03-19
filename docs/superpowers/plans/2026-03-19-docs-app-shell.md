# Docs App Shell Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a GitHub Pages compatible docs app shell in `docs/` that renders `language-tour.md` and `std.md` through a shared layout with lightweight snippet interaction.

**Architecture:** Keep `docs/language-tour.md`, `docs/std.md`, and `docs/examples/manifest.json` as the source of truth. Add a dependency-free static app in `docs/` with `index.html`, `styles.css`, `app.js`, and `.nojekyll`, then lock down the browser-facing contract with lightweight Rust integration tests plus a manual browser smoke pass.

**Tech Stack:** Static HTML, vanilla JavaScript, CSS, GitHub Pages, Rust integration tests in `crates/arktc/tests`

---

## File Structure

- Create: `docs/index.html`
  Shared shell entrypoint with sidebar, content region, and interaction pane
- Create: `docs/styles.css`
  Shared layout, typography, snippet card, and responsive styles
- Create: `docs/app.js`
  Hash router, markdown loader, minimal renderer, manifest normalization, snippet selection, and stub interaction pane
- Create: `docs/.nojekyll`
  Prevent Jekyll processing so raw markdown and JSON assets are fetchable on GitHub Pages
- Create: `crates/arktc/tests/docs_site.rs`
  Static contract tests for the docs app assets and browser path assumptions
- Do not modify unless required by a discovered bug:
  - `docs/language-tour.md`
  - `docs/std.md`
  - `docs/examples/manifest.json`

### Task 1: Lock Down The Static Docs-Site Contract

**Files:**
- Create: `crates/arktc/tests/docs_site.rs`

- [ ] **Step 1: Write a failing docs-site contract test for required files**

Write a test that checks for the presence of:

```rust
#[test]
fn docs_site_assets_exist() {
    assert!(repo_root().join("docs/index.html").exists());
    assert!(repo_root().join("docs/styles.css").exists());
    assert!(repo_root().join("docs/app.js").exists());
    assert!(repo_root().join("docs/.nojekyll").exists());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p arktc --test docs_site docs_site_assets_exist -- --exact`
Expected: FAIL because the new docs-site assets do not exist yet

- [ ] **Step 3: Add a second failing test for browser-facing path rules**

Extend `crates/arktc/tests/docs_site.rs` with a test that asserts:

- `docs/index.html` references only relative local assets such as `./styles.css` and `./app.js`
- `docs/app.js` contains route mappings for `language-tour` and `std`
- `docs/app.js` loads `./language-tour.md`, `./std.md`, and `./examples/manifest.json`
- `docs/app.js` normalizes manifest asset paths that start with `docs/`
- `docs/app.js` exposes `Run`, `Check`, `Build`, and `Test` as shell actions
- `docs/index.html` and `docs/app.js` do not hardcode absolute `http://`, `https://`, or root-relative `/` asset fetches
- `docs/app.js` does not browser-fetch `docs/...` verbatim

Suggested skeleton:

```rust
#[test]
fn docs_site_uses_relative_assets_and_known_routes() {
    let html = fs::read_to_string(repo_root().join("docs/index.html")).unwrap();
    let js = fs::read_to_string(repo_root().join("docs/app.js")).unwrap();

    assert!(html.contains("./styles.css"));
    assert!(html.contains("./app.js"));
    assert!(js.contains("language-tour"));
    assert!(js.contains("std"));
    assert!(js.contains("./language-tour.md"));
    assert!(js.contains("./std.md"));
    assert!(js.contains("./examples/manifest.json"));
    assert!(js.contains("Run"));
    assert!(js.contains("Check"));
    assert!(js.contains("Build"));
    assert!(js.contains("Test"));
    assert!(!html.contains("href=\"/"));
    assert!(!html.contains("src=\"/"));
    assert!(!js.contains("fetch(\"docs/"));
    assert!(!js.contains("fetch('/"));
    assert!(!js.contains("fetch(\"http"));
}
```

- [ ] **Step 4: Run the full docs-site test target and confirm it fails**

Run: `cargo test -p arktc --test docs_site`
Expected: FAIL because the docs-site assets and route loader do not exist yet

- [ ] **Step 5: Commit the failing test scaffold**

```bash
git add crates/arktc/tests/docs_site.rs
git commit -m "test: lock down docs site shell contract"
```

### Task 2: Create The Shared Static Shell

**Files:**
- Create: `docs/index.html`
- Create: `docs/styles.css`
- Create: `docs/.nojekyll`
- Test: `crates/arktc/tests/docs_site.rs`

- [ ] **Step 1: Write the minimal shell markup**

Create `docs/index.html` with:

- a left sidebar navigation region with:
  - product label
  - short project summary
  - primary entries for `Language Tour` and `Standard Surface`
  - placeholder entries for `Examples` and `Reference`
- a main content region for rendered markdown with a hero/header container
- a right interaction pane
- relative asset links only

Suggested structure:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Arukellt Docs</title>
    <link rel="stylesheet" href="./styles.css">
  </head>
  <body>
    <div class="app-shell">
      <aside id="sidebar"></aside>
      <main id="doc-view"></main>
      <section id="playground-pane"></section>
    </div>
    <script type="module" src="./app.js"></script>
  </body>
</html>
```

- [ ] **Step 2: Add the base visual system**

Create `docs/styles.css` with:

- a three-column layout
- active nav state
- snippet card styling
- plain code block fallback styling
- right-pane result styling
- responsive collapse for narrow screens

Start with the layout primitives first. Do not style markdown details before the shell spacing works.

- [ ] **Step 3: Add `.nojekyll`**

Create an empty `docs/.nojekyll` file.

- [ ] **Step 4: Run the asset-existence test and confirm it passes**

Run: `cargo test -p arktc --test docs_site docs_site_assets_exist -- --exact`
Expected: PASS

- [ ] **Step 5: Commit the shared shell assets**

```bash
git add docs/index.html docs/styles.css docs/.nojekyll
git commit -m "feat: add docs app shell assets"
```

### Task 3: Implement Hash Routing, Markdown Rendering, And Snippet State

**Files:**
- Create: `docs/app.js`
- Modify: `crates/arktc/tests/docs_site.rs`

- [ ] **Step 1: Extend the test to cover route and manifest contract details**

Add or tighten assertions in `crates/arktc/tests/docs_site.rs` so the test checks for:

- default route behavior for `language-tour`
- support for `#/language-tour` and `#/std`
- fetch targets `./language-tour.md`, `./std.md`, and `./examples/manifest.json`
- browser-side normalization of manifest paths that start with `docs/`
- presence of a `Test` action alongside `Run`, `Check`, and `Build`
- absence of root-relative and absolute browser asset fetches

- [ ] **Step 2: Run the test to verify it fails on the missing logic**

Run: `cargo test -p arktc --test docs_site docs_site_uses_relative_assets_and_known_routes -- --exact`
Expected: FAIL because `docs/app.js` does not yet satisfy the browser contract

- [ ] **Step 3: Implement the app state and route loader**

Create `docs/app.js` with:

- a route table:

```js
const ROUTES = {
  "language-tour": "./language-tour.md",
  std: "./std.md",
};
```

- hash parsing with default fallback to `language-tour`
- manifest loading from `./examples/manifest.json`
- path normalization that strips a leading `docs/` segment for browser fetches
- startup render on `DOMContentLoaded`
- rerender on `hashchange`

- [ ] **Step 4: Implement the minimal markdown renderer**

Support only:

- `#`, `##`, `###`
- paragraphs
- unordered lists
- inline code
- fenced code blocks
- `<!-- snippet: id -->`

Render:

- a hero/header block for the current document
- snippet-bound blocks as interactive cards with `data-snippet-id`
- non-snippet fenced blocks as plain code blocks
- orphan snippet markers as ignored markers rather than fatal parse errors

- [ ] **Step 5: Implement snippet selection and the stub interaction pane**

The right pane should show:

- overview state when nothing is selected
- selected snippet metadata
- action buttons: `Run`, `Check`, `Build`, `Test`
- stub result cards based on manifest mode:
  - `run` -> stdout fixture
  - `check_ok` -> success card
  - `check_fail` -> diagnostic card
  - `build_*_ok` -> build success card
  - `build_*_fail` -> unsupported/failure card
  - `test_ok` -> inline-test success card

- [ ] **Step 6: Implement fail-soft manifest and snippet error handling**

Handle all of these without breaking document rendering:

- manifest fetch failure -> render markdown, disable snippet actions, show a non-fatal notice
- manifest JSON parse failure -> render markdown, show a non-fatal notice
- missing manifest entry for a snippet id -> render the snippet card and show a warning state
- manifest paths that start with `docs/` -> normalize before browser fetch
- normalization or fixture fetch failure -> show a non-fatal asset warning in the pane

- [ ] **Step 7: Run a real browser smoke immediately after implementing the runtime**

Start a static server:

```bash
python3 -m http.server 8000 --directory docs
```

Then verify in a browser or Playwright against:

- `http://localhost:8000/`
- `http://localhost:8000/#/language-tour`
- `http://localhost:8000/#/std`

Check:

- `http://localhost:8000/` redirects or falls back to the `language-tour` view
- hash routing actually switches documents
- markdown fetch succeeds under browser path rules
- no request accidentally targets `docs/docs/...`
- selecting a snippet updates the right pane
- `Run`, `Check`, `Build`, and `Test` all switch pane state
- missing or unsupported snippet metadata degrades without a page crash
- manifest fetch failure degrades with a non-fatal notice instead of a blank page or crash:
  temporarily point the manifest fetch path at `./examples/missing.json` in devtools and reload
- manifest JSON parse failure degrades with a non-fatal notice instead of a blank page or crash:
  temporarily replace the loaded manifest response with invalid JSON and reload
- missing manifest entries degrade with a warning state instead of a blank page or crash:
  temporarily remove one snippet entry from the in-memory manifest in devtools and reselect that snippet

- [ ] **Step 8: Run the docs-site test target and make it pass**

Run: `cargo test -p arktc --test docs_site`
Expected: PASS

- [ ] **Step 9: Commit the docs app runtime**

```bash
git add docs/app.js crates/arktc/tests/docs_site.rs
git commit -m "feat: implement markdown-driven docs app shell"
```

### Task 4: Protect Existing Docs Contracts And Verify The Shell End-To-End

**Files:**
- Verify: `crates/arktc/tests/docs.rs`
- Verify: `crates/chef/tests/docs.rs`
- Verify: `crates/arktc/tests/readme.rs`
- Verify: `crates/chef/tests/readme.rs`
- Verify: `crates/arktfmt/tests/readme.rs`
- Verify: `docs/index.html`
- Verify: `docs/app.js`

- [ ] **Step 1: Re-run the docs contract tests**

Run:

```bash
cargo test -p arktc --test docs
cargo test -p chef --test docs
```

Expected: PASS

- [ ] **Step 2: Re-run README smoke tests**

Run:

```bash
cargo test -p arktc --test readme
cargo test -p chef --test readme
cargo test -p arktfmt --test readme
```

Expected: PASS

- [ ] **Step 3: Run formatting and full test suite**

Run:

```bash
cargo fmt --check
cargo test -q
```

Expected: PASS

- [ ] **Step 4: Run the benchmark smoke**

Run: `cargo run -q -p chef -- benchmark benchmarks/pure_logic.json`
Expected: JSON result with `"passed": 5` and `"version": "v0.1"`

- [ ] **Step 5: Final manual browser smoke on the static site**

Serve `docs/` locally:

```bash
python3 -m http.server 8000 --directory docs
```

Then verify in a browser:

- `http://localhost:8000/`
- `http://localhost:8000/#/language-tour`
- `http://localhost:8000/#/std`

Check:

- no-hash entry falls back to the `language-tour` view
- both routes render
- sidebar route highlight changes
- sidebar shows product label, project summary, and placeholder entries for `Examples` and `Reference`
- main pane shows a hero/header and plain code blocks still render when not snippet-bound
- clicking a snippet card updates the right pane
- `Run`, `Check`, `Build`, and `Test` switch stub output
- no fetch path accidentally points at `docs/docs/...`
- induced manifest fetch failure, JSON parse failure, and missing manifest entry cases show non-fatal UI notices

- [ ] **Step 6: Commit the verified docs shell**

```bash
git add docs/index.html docs/styles.css docs/app.js docs/.nojekyll crates/arktc/tests/docs_site.rs
git commit -m "feat: add GitHub Pages docs app shell"
```
