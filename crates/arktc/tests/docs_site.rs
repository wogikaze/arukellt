use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn docs_site_assets_exist() {
    assert!(repo_root().join("docs/index.html").exists());
    assert!(repo_root().join("docs/styles.css").exists());
    assert!(repo_root().join("docs/app.js").exists());
    assert!(repo_root().join("docs/.nojekyll").exists());
}

#[test]
fn docs_site_uses_relative_assets_and_known_routes() {
    let html =
        fs::read_to_string(repo_root().join("docs/index.html")).expect("read docs/index.html");
    let js = fs::read_to_string(repo_root().join("docs/app.js")).expect("read docs/app.js");

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
    assert!(js.contains("Manifest doc unavailable"));
    assert!(!html.contains("href=\"/"));
    assert!(!html.contains("src=\"/"));
    assert!(!js.contains("fetch(\"docs/"));
    assert!(!js.contains("fetch('/"));
    assert!(!js.contains("fetch(\"http"));
}

#[test]
fn docs_site_styles_define_responsive_breakpoints() {
    let css =
        fs::read_to_string(repo_root().join("docs/styles.css")).expect("read docs/styles.css");

    assert!(css.contains("@media"));
    assert!(css.contains("max-width"));
    assert!(css.contains("grid-template-columns"));
}

#[test]
fn docs_site_runtime_helpers_work_in_node() {
    let script = r#"
import { pathToFileURL } from 'node:url';

const moduleUrl = pathToFileURL(process.cwd() + '/docs/app.js').href;
const elements = new Map([
  ['sidebar', { innerHTML: '', listeners: {}, addEventListener(type, cb) { this.listeners[type] = cb; } }],
  ['doc-view', { innerHTML: '', listeners: {}, addEventListener(type, cb) { this.listeners[type] = cb; } }],
  ['playground-pane', { innerHTML: '', listeners: {}, addEventListener(type, cb) { this.listeners[type] = cb; } }],
]);
const listeners = new Map();

class FakeElement {}

globalThis.Element = FakeElement;

globalThis.document = {
  title: '',
  addEventListener(type, callback) {
    listeners.set(type, callback);
  },
  getElementById(id) {
    return elements.get(id) ?? null;
  },
};

globalThis.window = {
  addEventListener() {},
};

globalThis.location = { hash: '' };
globalThis.history = {
  replaceState(_state, _title, hash) {
    globalThis.location.hash = hash;
  },
};

const fixtures = {
  './language-tour.md': `# Arukellt Language Tour\n\nA small intro paragraph.\n\n## Hello World\n\n<!-- snippet: language-tour-hello-world -->\n\`\`\`arukel\nfn main():\n  42\n\`\`\`\n`,
  './examples/manifest.json': JSON.stringify([
    {
      id: 'language-tour-hello-world',
      doc: 'docs/language-tour.md',
      fixture: 'docs/examples/language-tour/01-hello-world.ar',
      mode: 'run',
      stdout_fixture: 'docs/examples/language-tour/01-hello-world.stdout',
    },
  ]),
  './examples/language-tour/01-hello-world.stdout': 'Hello, world!\\n',
};

globalThis.fetch = async (path) => {
  if (!(path in fixtures)) {
    return { ok: false, status: 404, text: async () => '' };
  }

  return {
    ok: true,
    status: 200,
    text: async () => fixtures[path],
  };
};

const mod = await import(moduleUrl);

if (mod.resolveRouteFromHash('') !== 'language-tour') {
  throw new Error('default route should fall back to language-tour');
}

if (mod.resolveRouteFromHash('#/std') !== 'std') {
  throw new Error('std route should resolve');
}

if (mod.normalizeBrowserAssetPath('docs/examples/std/01-closure-map.stdout') !== './examples/std/01-closure-map.stdout') {
  throw new Error('docs/ paths should be normalized for browser fetches');
}

if (mod.normalizeBrowserAssetPath('/docs/app.js') !== null) {
  throw new Error('root-relative paths should be rejected');
}

if (mod.normalizeBrowserAssetPath('../outside.md') !== null) {
  throw new Error('parent-directory asset paths should be rejected');
}

await listeners.get('DOMContentLoaded')();
await new Promise((resolve) => setTimeout(resolve, 0));

if (globalThis.location.hash !== '#/language-tour') {
  throw new Error('startup should canonicalize the empty hash to #/language-tour');
}

if (!elements.get('sidebar').innerHTML.includes('#/std')) {
  throw new Error('startup should render sidebar navigation');
}

if (!elements.get('doc-view').innerHTML.includes('Arukellt Language Tour')) {
  throw new Error('startup should render the selected markdown document');
}

if (!elements.get('playground-pane').innerHTML.includes('Snippet overview')) {
  throw new Error('startup should render the interaction pane');
}

const snippetTarget = new FakeElement();
snippetTarget.closest = (selector) => {
  if (selector !== '[data-snippet-id]') return null;
  return {
    getAttribute(name) {
      return name === 'data-snippet-id' ? 'language-tour-hello-world' : null;
    },
  };
};

elements.get('doc-view').listeners.click({ target: snippetTarget });

if (!elements.get('playground-pane').innerHTML.includes('Run output')) {
  throw new Error('snippet selection should update the interaction pane');
}

const actionTarget = new FakeElement();
actionTarget.closest = (selector) => {
  if (selector !== '[data-action]') return null;
  return {
    getAttribute(name) {
      return name === 'data-action' ? 'run' : null;
    },
    hasAttribute() {
      return false;
    },
  };
};

elements.get('playground-pane').listeners.click({ target: actionTarget });

if (!elements.get('playground-pane').innerHTML.includes('Run output')) {
  throw new Error('action clicks should keep the pane in the selected action state');
}

const parsed = mod.parseMarkdown(`
# Title

<!-- snippet: hello -->

\`\`\`arukel
fn main():
  42
\`\`\`
`, 'language-tour');

if (parsed.blocks.some((block) => block.kind === 'snippet')) {
  throw new Error('snippet marker must be ignored when a blank line separates it from the code fence');
}

const indexed = mod.buildManifestIndex([
  { id: 'dup', doc: 'docs/language-tour.md', fixture: 'docs/examples/a.ar', mode: 'check_ok' },
  { id: 'dup', doc: 'docs/std.md', fixture: 'docs/examples/b.ar', mode: 'check_ok' },
  { doc: 'docs/std.md', fixture: 'docs/examples/c.ar', mode: 'run' },
]);

if (!indexed.warnings.some((warning) => warning.includes('Duplicate manifest id'))) {
  throw new Error('duplicate manifest ids should emit a warning');
}

if (!indexed.warnings.some((warning) => warning.includes('missing a string id'))) {
  throw new Error('manifest entries without string ids should emit a warning');
}

if (indexed.entries.length !== 1) {
  throw new Error('invalid or duplicate manifest entries should be skipped');
}

const malformed = mod.buildManifestIndex([
  { id: 'bad-path', doc: '/docs/language-tour.md', fixture: 'docs/examples/a.ar', mode: 'run' },
]);

if (!malformed.warnings.some((warning) => warning.includes('invalid browser asset paths'))) {
  throw new Error('invalid manifest asset paths should emit a warning');
}

if (malformed.entries.length !== 1) {
  throw new Error('entries with invalid browser asset paths should remain available to the UI');
}
"#;

    let output = Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(script)
        .current_dir(repo_root())
        .output()
        .expect("run node docs site smoke");

    assert!(
        output.status.success(),
        "expected node docs-site smoke to pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
