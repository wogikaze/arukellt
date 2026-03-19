# Docs App Shell Design

## Summary

Build a GitHub Pages compatible docs app shell in `docs/` for Arukellt `v0.0.1`.
The shell should present the existing markdown documents as a public-facing documentation site without changing those markdown files into app-owned content.

This work is explicitly about the page shell, routing, layout, and lightweight interaction model.
The actual long-form documentation content will be generated in a later step.

## Goals

- Publish documentation from the repository's `docs/` directory through GitHub Pages
- Keep `docs/language-tour.md` and `docs/std.md` as the source of truth for document content
- Provide a shared docs shell for both pages
- Support route switching between `Language Tour` and `Standard Surface`
- Provide a minimal interactive panel for snippet-oriented actions such as `Run`, `Check`, and `Build`
- Keep the implementation static and dependency-free so it works on GitHub Pages without a build step

## Non-Goals

- Full playground execution inside the browser
- A general-purpose markdown engine
- Search, version switching, or package reference navigation
- Replacing the existing Rust test suite as the source of truth for compiler/runtime behavior
- Generating the final prose content for the docs pages in this step

## Constraints

- GitHub Pages serves the repository's `docs/` directory directly
- The app must work with static files only
- Paths must be relative so the site works under a repository subpath
- The Pages deployment must bypass Jekyll processing for raw asset delivery
- No bundler, framework, or server-side preprocessing
- Existing docs-as-tests must keep passing
- The current markdown files and `docs/examples/manifest.json` remain the contract for snippet identity and behavior

## Proposed Architecture

### Files

- `docs/index.html`
  Shared application shell and root DOM structure
- `docs/styles.css`
  Shared docs layout and component styling
- `docs/app.js`
  Hash routing, markdown loading, snippet rendering, and interactive pane state
- `docs/language-tour.md`
  Existing source document
- `docs/std.md`
  Existing source document
- `docs/examples/manifest.json`
  Existing snippet metadata contract
- `docs/.nojekyll`
  Ensures GitHub Pages deploys the static assets directly without a Jekyll transform

### Routing

Use hash routing so GitHub Pages can serve one static entrypoint:

- `#/language-tour`
- `#/std`

If no hash is present, default to `#/language-tour`.

### Source of Truth

Document prose and code examples remain in markdown.
The app reads markdown and manifest data at runtime and renders a documentation view.
The shell must not duplicate snippet bodies into JavaScript constants.

### Path Resolution Contract

The browser app must not fetch repo-root relative manifest strings verbatim.
The current manifest exists primarily for repository-local tests and uses paths such as `docs/...`.

The app should resolve assets in one of these two ways:

- Use route-to-document mapping in the app for top-level docs:
  - `#/language-tour` -> `./language-tour.md`
  - `#/std` -> `./std.md`
- Normalize manifest-owned asset paths before fetching them in the browser by stripping the leading `docs/` segment and then resolving relative to `docs/index.html`

This keeps the existing Rust tests unchanged while making the browser contract valid under GitHub Pages.

## Layout

Use a shared three-column docs shell.

### Left Sidebar

- Product label and short project summary
- Primary navigation entries:
  - `Language Tour`
  - `Standard Surface`
- Secondary placeholder entries for future expansion:
  - `Examples`
  - `Reference`
- Visual route highlighting

### Main Content

- Hero header derived from the selected document
- Rendered markdown sections
- Styled snippet cards for fenced code blocks that are associated with a snippet id
- Plain code blocks for any fenced block without a snippet marker

### Right Interaction Pane

- Selected snippet title
- Snippet metadata such as document id and execution mode
- Action buttons:
  - `Run`
  - `Check`
  - `Build`
  - `Test`
- Stubbed result panel that changes based on the selected snippet and selected action

On small screens, the layout collapses to a single-column document-first view, with the interaction pane rendered beneath the active content.

## Markdown Rendering Scope

Implement only the subset needed by the current docs:

- `#`, `##`, `###`
- paragraphs
- unordered lists
- fenced code blocks
- inline code
- snippet marker comments in the form `<!-- snippet: ... -->`

This is intentionally not a full markdown parser.
If the docs later outgrow this subset, the rendering layer can be extended deliberately.

## Snippet Binding Model

The shell binds markdown snippets to manifest entries using the existing marker convention:

1. Read a markdown file
2. Detect `<!-- snippet: <id> -->`
3. Bind the immediately following fenced code block to that snippet id
4. Look up that id in `docs/examples/manifest.json`
5. Render the block as an interactive snippet card

The app should fail soft if a snippet id is missing from the manifest:

- Render the snippet anyway
- Show a warning badge or non-blocking message in the interaction pane
- Do not break route rendering

## Interaction Model

### Initial State

- Route loads a document
- No snippet is selected yet
- The interaction pane shows an overview card describing available actions

### Snippet Selection

- Clicking a snippet card selects it
- The interaction pane updates to the selected snippet
- The selected card gets a visual active state

### Action Behavior

The interaction pane is intentionally stubbed for now.
Actions should switch the pane to a plausible result view derived from manifest data:

- `run`
  - show stdout-style output using `stdout_fixture`
- `check_ok`
  - show a success state
- `check_fail`
  - show a diagnostic card using `error_code`
- `build_wasi_ok` / `build_js_ok`
  - show a build success card
- `build_wasi_fail` / `build_js_fail`
  - show an unsupported or failure card using `error_substring`
- `test_ok`
  - show an inline test success card

Buttons that do not make sense for the current snippet mode can stay visible but inactive, or visible and informational, as long as the behavior is clear.

## Visual Direction

The shell should feel like a deliberate product docs surface, not a default markdown dump.

- light theme by default
- warm off-white page background
- white content surfaces
- restrained slate/ink text colors
- clear contrast between prose and snippet cards
- subtle emphasis on the active snippet and active route
- code surfaces that feel interactive rather than purely decorative

The design should avoid over-styled marketing aesthetics and avoid looking like raw GitHub markdown.

## Error Handling

The app should degrade gracefully if static assets are missing or malformed.

- If markdown fetch fails, show a document load error state in the main pane
- If the manifest fetch fails, still render markdown and disable snippet interaction
- If a manifest path starts with `docs/`, normalize it before browser fetch; if normalization still fails, show a non-fatal asset warning
- If a snippet marker exists without a following fenced code block, ignore the marker and continue
- If JSON parsing fails, show a non-fatal notice in the interaction pane

## Testing Strategy

The Rust docs-as-tests remain the correctness layer for snippet content and executable contracts.
The new docs shell is presentation-only and should not duplicate those semantic checks.

Expected validation for this shell:

- existing Rust docs tests continue to pass
- the shell uses only relative asset paths
- the shell reads `language-tour.md`, `std.md`, and `examples/manifest.json`
- route switching works without server-side support
- missing snippet metadata does not crash the page

## Open Extension Path

This design intentionally leaves room for a later phase that:

- expands the markdown prose
- connects the interaction pane to a real runner or playground API
- adds more docs sections without changing the shell architecture

## Acceptance Criteria

- GitHub Pages can serve the docs app directly from `docs/`
- `docs/index.html` renders a shared shell for both documents
- `#/language-tour` and `#/std` both work
- the shell loads markdown from the existing files instead of duplicating the content
- snippet markers are turned into interactive snippet cards
- the right-side interaction pane responds to snippet selection with stub output
- existing docs/test contracts remain green
