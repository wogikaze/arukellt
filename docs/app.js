// ============================================================
//  Arukellt Docs — SPA
// ============================================================

// ── WASM interpreter (lang-playground-core) ──────────────────
// Loaded lazily on first Playground visit; falls back to the
// built-in JS interpreter if the WASM bundle is unavailable.
let _wasmMod = null;

async function loadWasm() {
  if (_wasmMod) return _wasmMod;
  try {
    const mod = await import('./pkg/lang_playground_core.js');
    await mod.default(); // init wasm
    _wasmMod = mod;
    return mod;
  } catch (e) {
    console.warn('[playground] WASM unavailable, using JS fallback:', e);
    return null;
  }
}

function runWithWasm(mod, src) {
  try {
    const json = mod.run_program(src);
    const res = JSON.parse(json);
    if (res.ok) {
      const out = (res.output || '').trimEnd();
      const val = res.value;
      if (out) return { lines: out.split('\n'), ok: true };
      if (val && val.type !== 'unit') return { lines: [JSON.stringify(val)], ok: true };
      return { lines: [], ok: true };
    }
    return { lines: (res.errors || ['unknown error']).map(e => 'Error: ' + e), ok: false };
  } catch (e) {
    return { lines: ['Error: ' + e.message], ok: false };
  }
}

// ── Router ───────────────────────────────────────────────────
function route() {
  const hash = location.hash.replace(/^#\/?/, '');
  updateNav(hash);
  const app = document.getElementById('app');
  if (!hash || hash === '') renderHome(app);
  else if (hash === 'docs/tour') renderDocs(app, 'language-tour.md', tourSidebar());
  else if (hash === 'docs/std')  renderDocs(app, 'std.md', stdSidebar());
  else if (hash === 'playground') renderPlayground(app);
  else renderHome(app);
}

function updateNav(hash) {
  document.querySelectorAll('.nav-link').forEach(el => {
    const r = el.dataset.route || '';
    el.classList.toggle('active', hash.startsWith(r) && r !== '');
  });
}

window.addEventListener('hashchange', route);
window.addEventListener('DOMContentLoaded', route);

// ── Syntax Highlighter (Arukellt / arukel) ───────────────────
const KEYWORDS  = /\b(fn|if|else|match|import|type|let|in|true|false)\b/g;
const FN_NAME   = /\bfn\s+(\w+)/g;
const TYPES     = /\b(Int|Bool|String|i64|Fn|Result|Ok|Err|Seq|Iter|List|Void)\b/g;
const ADT_CTOR  = /\b([A-Z][a-zA-Z0-9_]*)\b/g;
const NUMBERS   = /\b(\d+)\b/g;
const STRINGS   = /"([^"\\]|\\.)*"/g;
const COMMENTS  = /\/\/.*/g;
const OPERATORS = /(\|>|->|\.\.=|==|!=|<=|>=|&&|\|\||[+\-*\/%<>!])/g;
const BUILTINS  = /\b(console|string|parse|fs|stdin|iter|len|ends_with_at|strip_suffix)\b/g;

function highlightArukel(code) {
  // escape HTML first
  let s = code.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');

  // Order matters: comments > strings > keywords > ...
  // Use placeholder approach to avoid re-processing
  const parts = [];
  let result = '';

  function protect(html, cls) {
    const key = `\x00${parts.length}\x00`;
    parts.push(`<span class="tok-${cls}">${html}</span>`);
    return key;
  }

  // Comments
  s = s.replace(/\/\/.*/g, m => protect(m, 'cmt'));
  // Strings
  s = s.replace(/"([^"\\]|\\.)*"/g, m => protect(m, 'str'));
  // Keywords
  s = s.replace(/\b(fn|if|else|match|import|type|let|in|true|false)\b/g, m => protect(m, 'kw'));
  // Built-in types
  s = s.replace(/\b(Int|Bool|String|i64|Fn|Result|Seq|Iter|List)\b/g, m => protect(m, 'type'));
  // ADT constructors (capital letter start, not already matched)
  s = s.replace(/\b([A-Z][a-zA-Z0-9_]*)\b/g, m => protect(m, 'adt'));
  // Builtins
  s = s.replace(/\b(console|string|parse|fs|stdin|iter|len|ends_with_at|strip_suffix)\b/g, m => protect(m, 'builtin'));
  // Numbers
  s = s.replace(/\b(\d+)\b/g, m => protect(m, 'num'));
  // Operators
  s = s.replace(/(\|>|->|\.\.=|==|!=|<=|>=|&&|\|\||[+\-*\/%<>!])/g, m => protect(m, 'op'));

  // Restore placeholders
  s = s.replace(/\x00(\d+)\x00/g, (_, i) => parts[+i]);
  return s;
}

// ── Markdown rendering ────────────────────────────────────────
function renderMarkdown(md) {
  const renderer = new marked.Renderer();
  renderer.code = (code, lang) => {
    const highlighted = (lang === 'arukel' || lang === 'arukellt')
      ? highlightArukel(code)
      : (hljs.getLanguage(lang)
          ? hljs.highlight(code, { language: lang }).value
          : hljs.highlightAuto(code).value);
    return `<pre><code class="language-${lang||''}">${highlighted}</code></pre>`;
  };
  // Strip <!-- snippet: ... --> comments silently
  const clean = md.replace(/<!--\s*snippet:\s*\S+\s*-->/g, '');
  marked.setOptions({ renderer, gfm: true, breaks: false });
  let html = marked.parse(clean);
  // Enhance yes/no in table cells
  html = html.replace(/<td>yes<\/td>/gi, '<td><span class="badge-yes">yes</span></td>');
  html = html.replace(/<td>no<\/td>/gi,  '<td><span class="badge-no">no</span></td>');
  return html;
}

// ── Home page ─────────────────────────────────────────────────
function renderHome(app) {
  document.title = 'Arukellt';
  app.innerHTML = `
<div class="home-wrap">
  <section class="hero">
    <div class="hero-text">
      <p class="hero-eyebrow">// expression-first · v0.0.1</p>
      <h1 class="hero-title">arukellt<em>.</em></h1>
      <p class="hero-desc">
        An expression-first, indentation-sensitive language aimed at small,
        recoverable programs. Static types, pattern matching, and a clean
        functional pipeline syntax.
      </p>
      <div class="hero-actions">
        <a href="#docs/tour" class="btn btn-primary">Language Tour</a>
        <a href="#playground" class="btn btn-ghost">Try in Playground</a>
      </div>
    </div>
    <div class="hero-code">
      <div class="hero-code-bar">
        <span class="code-dot code-dot-r"></span>
        <span class="code-dot code-dot-y"></span>
        <span class="code-dot code-dot-g"></span>
        <span class="hero-code-label">fizz_buzz.ar</span>
      </div>
      <pre><code>${highlightArukel(
`import console

fn fizz_buzz_label(n: i64) -> String:
  if divisible_by(n, 15):
    "FizzBuzz"
  else:
    if divisible_by(n, 3):
      "Fizz"
    else:
      if divisible_by(n, 5):
        "Buzz"
      else:
        string(n)

fn divisible_by(value: i64, divisor: i64) -> Bool:
  value % divisor == 0

fn main():
  (1..=100)
    .map(fizz_buzz_label)
    .join("\\n")
    |> console.println`)}</code></pre>
    </div>
  </section>

  <section class="features">
    <div class="feature">
      <div class="feature-badge">static types</div>
      <div class="feature-title">No runtime surprises</div>
      <div class="feature-desc">Fully inferred types with structured diagnostics. Errors include a suggested fix and a machine-readable JSON format.</div>
    </div>
    <div class="feature">
      <div class="feature-badge">expression-first</div>
      <div class="feature-title">Everything is a value</div>
      <div class="feature-desc"><code>if</code>, <code>match</code>, and function bodies are all expressions. Indentation scopes blocks cleanly.</div>
    </div>
    <div class="feature">
      <div class="feature-badge">wasm targets</div>
      <div class="feature-title">Compiles to WebAssembly</div>
      <div class="feature-desc">Build for <code>wasm-js</code> or <code>wasm-wasi</code>. List pipelines, closures, and iterators all lower correctly.</div>
    </div>
  </section>

  <div class="cards">
    <a href="#docs/tour" class="card">
      <div class="card-icon">📖</div>
      <div class="card-label">// start_here</div>
      <div class="card-title">Language Tour</div>
      <div class="card-desc">
        Learn the syntax, type system, ADTs, pattern matching, and the
        expression-first evaluation model.
      </div>
      <span class="card-link">Read the tour →</span>
    </a>
    <a href="#docs/std" class="card">
      <div class="card-icon">📚</div>
      <div class="card-label">// std_surface</div>
      <div class="card-title">Standard Library</div>
      <div class="card-desc">
        Full target support matrix — which builtins work on the interpreter,
        <code>wasm-js</code>, and <code>wasm-wasi</code>.
      </div>
      <span class="card-link">Browse the std →</span>
    </a>
    <a href="#playground" class="card">
      <div class="card-icon">⚡</div>
      <div class="card-label">// interactive</div>
      <div class="card-title">Playground</div>
      <div class="card-desc">
        Edit and run Arukellt programs directly in your browser. Choose from
        bundled examples or write your own.
      </div>
      <span class="card-link">Open playground →</span>
    </a>
  </div>
</div>`;
}

// ── Docs page ─────────────────────────────────────────────────
function tourSidebar() {
  return `
    <div class="sidebar-section">
      <div class="sidebar-label">// getting_started</div>
      <a class="sidebar-link" href="#docs/tour">Introduction</a>
    </div>
    <div class="sidebar-section">
      <div class="sidebar-label">// language</div>
      <a class="sidebar-link" href="#docs/tour">Hello World</a>
      <a class="sidebar-link" href="#docs/tour">Pure Functions</a>
      <a class="sidebar-link" href="#docs/tour">ADTs and Match</a>
      <a class="sidebar-link" href="#docs/tour">Structured Diagnostics</a>
    </div>
    <div class="sidebar-section">
      <div class="sidebar-label">// toolchain</div>
      <a class="sidebar-link" href="#docs/std">arktc check</a>
      <a class="sidebar-link" href="#docs/std">chef run</a>
      <a class="sidebar-link" href="#docs/std">arktc build</a>
      <a class="sidebar-link" href="#docs/std">arkli (REPL)</a>
    </div>`;
}

function stdSidebar() {
  return `
    <div class="sidebar-section">
      <div class="sidebar-label">// overview</div>
      <a class="sidebar-link" href="#docs/std">Target Matrix</a>
    </div>
    <div class="sidebar-section">
      <div class="sidebar-label">// collections</div>
      <a class="sidebar-link" href="#docs/std">Pipelines &amp; Closures</a>
      <a class="sidebar-link" href="#docs/std">File Reads</a>
      <a class="sidebar-link" href="#docs/std">Inline Tests</a>
    </div>
    <div class="sidebar-section">
      <div class="sidebar-label">// tooling</div>
      <a class="sidebar-link" href="#docs/std">Interactive REPL</a>
      <a class="sidebar-link" href="#docs/std">Chef Build</a>
    </div>
    <div class="sidebar-section">
      <div class="sidebar-label">// wasm</div>
      <a class="sidebar-link" href="#docs/std">WASM Boundary</a>
    </div>`;
}

function renderDocs(app, mdFile, sidebarHtml) {
  document.title = mdFile.replace('.md','') + ' — Arukellt';
  app.innerHTML = `
<div class="docs-layout">
  <nav class="sidebar">${sidebarHtml}</nav>
  <div class="docs-content">
    <div class="md-body" id="md-body"><div class="loading">loading…</div></div>
  </div>
</div>`;

  fetch(mdFile)
    .then(r => {
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      return r.text();
    })
    .then(md => {
      document.getElementById('md-body').innerHTML = renderMarkdown(md);
    })
    .catch(err => {
      document.getElementById('md-body').innerHTML =
        `<p class="dim">Could not load ${mdFile}: ${err.message}</p>`;
    });
}

// ── Playground examples ───────────────────────────────────────
const EXAMPLES = {
  'hello_world': {
    label: 'hello_world.ar',
    code: `import console

fn main():
  "Hello, world!" |> console.println`
  },
  'factorial': {
    label: 'factorial.ar',
    code: `import console

fn factorial(n: i64) -> i64:
  if n == 0:
    1
  else:
    n * factorial(n - 1)

fn main():
  factorial(10) |> string |> console.println`
  },
  'fibonacci': {
    label: 'fibonacci.ar',
    code: `import console

fn fibonacci(n: i64) -> i64:
  if n <= 1:
    n
  else:
    fibonacci(n - 1) + fibonacci(n - 2)

fn main():
  fibonacci(10) |> string |> console.println`
  },
  'fizz_buzz': {
    label: 'fizz_buzz.ar',
    code: `import console

fn fizz_buzz_label(n: i64) -> String:
  if divisible_by(n, 15):
    "FizzBuzz"
  else:
    if divisible_by(n, 3):
      "Fizz"
    else:
      if divisible_by(n, 5):
        "Buzz"
      else:
        string(n)

fn divisible_by(value: i64, divisor: i64) -> Bool:
  value % divisor == 0

fn main():
  (1..=20)
    .map(fizz_buzz_label)
    .join("\\n")
    |> console.println`
  },
  'map_filter_sum': {
    label: 'map_filter_sum.ar',
    code: `import console

fn main():
  [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    .map(double)
    .filter(is_divisible_by_three)
    .sum()
    |> string
    |> console.println

fn double(value: i64) -> i64:
  value * 2

fn is_divisible_by_three(value: i64) -> Bool:
  divisible_by(value, 3)

fn divisible_by(value: i64, divisor: i64) -> Bool:
  value % divisor == 0`
  },
  'closure': {
    label: 'closure.ar',
    code: `import console

fn make_adder(base: i64) -> Fn<i64, i64>:
  n -> base + n

fn main():
  make_adder(10)(32) |> string |> console.println`
  },
  'powers': {
    label: 'powers.ar',
    code: `import console

fn power(base: i64, exp: i64) -> i64:
  if exp == 0:
    1
  else:
    base * power(base, exp - 1)

fn power_of_two(exp: i64) -> i64:
  power(2, exp)

fn main():
  (0..=10)
    .map(power_of_two)
    .map(string)
    .join(", ")
    |> console.println`
  },
  'result': {
    label: 'result_error_handling.ar',
    code: `import console

fn divide(a: i64, b: i64) -> Result<i64, String>:
  if b == 0:
    Err("division by zero")
  else:
    Ok(a / b)

fn show(result: Result<i64, String>) -> String:
  match result:
    Ok(value) -> value |> string
    Err(msg) -> msg

fn main():
  divide(10, 0) |> show |> console.println`
  },
  'closure_map': {
    label: 'closure_map.ar (std)',
    code: `import console

fn main():
  [1, 2, 3]
    .map(n -> n * 2)
    .map(string)
    .join(", ")
    |> console.println`
  },
};

// ── Arukellt Interpreter ──────────────────────────────────────
//   Supports the subset used in playground examples.
const Arukellt = (() => {

  // ── Lexer ─────────────────────────────────────────────────
  function lex(src) {
    const toks = [];
    let i = 0;
    let lineIndents = []; // track indent per logical line

    while (i < src.length) {
      // Skip inline whitespace
      if (src[i] === ' ' || src[i] === '\t' || src[i] === '\r') { i++; continue; }

      // Newline → emit NL, then measure next indent
      if (src[i] === '\n') {
        toks.push({ t: 'NL', pos: i });
        i++;
        let ind = 0;
        while (i < src.length && (src[i] === ' ' || src[i] === '\t')) {
          ind += src[i] === '\t' ? 2 : 1;
          i++;
        }
        // skip blank lines / comment lines
        if (i < src.length && src[i] === '\n') continue;
        if (i < src.length && src[i] === '/' && src[i+1] === '/') {
          while (i < src.length && src[i] !== '\n') i++;
          continue;
        }
        toks.push({ t: 'INDENT_VAL', v: ind });
        continue;
      }

      // Comments
      if (src[i] === '/' && src[i+1] === '/') {
        while (i < src.length && src[i] !== '\n') i++;
        continue;
      }

      // Numbers
      if (/\d/.test(src[i])) {
        let s = '';
        while (i < src.length && /\d/.test(src[i])) s += src[i++];
        toks.push({ t: 'INT', v: parseInt(s, 10) });
        continue;
      }

      // Strings
      if (src[i] === '"') {
        i++;
        let s = '';
        while (i < src.length && src[i] !== '"') {
          if (src[i] === '\\') { i++; const c = src[i]; s += c === 'n' ? '\n' : c === 't' ? '\t' : c; }
          else s += src[i];
          i++;
        }
        i++;
        toks.push({ t: 'STR', v: s });
        continue;
      }

      // Identifiers & keywords
      if (/[a-zA-Z_]/.test(src[i])) {
        let s = '';
        while (i < src.length && /[a-zA-Z0-9_]/.test(src[i])) s += src[i++];
        const KW = { fn:1, if:1, else:1, import:1, type:1, match:1, let:1, in:1 };
        toks.push({ t: KW[s] ? s.toUpperCase() : (s === 'true' || s === 'false') ? 'BOOL' : 'ID', v: s });
        continue;
      }

      // Multi-char operators
      const two = src.slice(i, i+2);
      const three = src.slice(i, i+3);
      if (three === '..=') { toks.push({ t: 'DOTDOTEQ' }); i += 3; continue; }
      if (two === '|>') { toks.push({ t: 'PIPE' }); i += 2; continue; }
      if (two === '->') { toks.push({ t: 'ARROW' }); i += 2; continue; }
      if (two === '==') { toks.push({ t: 'EQ' }); i += 2; continue; }
      if (two === '!=') { toks.push({ t: 'NEQ' }); i += 2; continue; }
      if (two === '<=') { toks.push({ t: 'LTE' }); i += 2; continue; }
      if (two === '>=') { toks.push({ t: 'GTE' }); i += 2; continue; }
      if (two === '&&') { toks.push({ t: 'AND' }); i += 2; continue; }
      if (two === '||') { toks.push({ t: 'OR' }); i += 2; continue; }

      const singles = {'+':'ADD','-':'SUB','*':'MUL','/':'DIV','%':'MOD',
        '<':'LT','>':'GT','!':'NOT','(':'LP',')':'RP','[':'LB',']':'RB',
        ',':'COMMA',':':'COLON','.':'DOT','=':'ASSIGN'};
      if (singles[src[i]]) { toks.push({ t: singles[src[i]] }); i++; continue; }
      i++; // skip unknown
    }
    toks.push({ t: 'EOF' });
    return toks;
  }

  // ── Pre-process: build structured source ─────────────────
  // Strategy: split into top-level function definitions, then
  // collapse each body into a single flat line for parsing.
  function preprocess(src) {
    const lines = src.split('\n');
    const fns = {};
    const types = {};
    let curFn = null;
    let bodyLines = [];
    let baseIndent = 0;

    function flushFn() {
      if (curFn) {
        fns[curFn.name] = { ...curFn, body: joinBody(bodyLines, baseIndent + 2) };
        bodyLines = [];
      }
    }

    for (const rawLine of lines) {
      const stripped = rawLine.trimEnd();
      if (!stripped) continue;
      const trimmed = stripped.trimStart();
      if (trimmed.startsWith('//')) continue;
      const indent = stripped.length - trimmed.length;

      if (indent === 0) {
        flushFn();
        curFn = null;

        if (trimmed.startsWith('fn ')) {
          const h = parseFnHeader(trimmed);
          if (h) {
            const params = parseParamsStr(h.paramStr);
            curFn = { name: h.name, params };
            baseIndent = 0;
            if (h.inline) bodyLines = [h.inline];
            else bodyLines = [];
          }
        } else if (trimmed.startsWith('type ')) {
          const m = trimmed.match(/^type\s+(\w+)\s*=/);
          if (m) types[m[1]] = true;
        }
        // import → ignore
      } else {
        if (curFn) bodyLines.push({ indent, text: trimmed });
      }
    }
    flushFn();
    return { fns, types };
  }

  // Parse params string, respecting nested <> and ()
  function parseParamsStr(s) {
    if (!s.trim()) return [];
    const parts = [];
    let ad = 0, pd = 0, cur = '';
    for (const ch of s) {
      if      (ch === '<') { ad++; cur += ch; }
      else if (ch === '>') { ad--; cur += ch; }
      else if (ch === '(') { pd++; cur += ch; }
      else if (ch === ')') { pd--; cur += ch; }
      else if (ch === ',' && ad === 0 && pd === 0) { parts.push(cur.trim()); cur = ''; }
      else cur += ch;
    }
    if (cur.trim()) parts.push(cur.trim());
    return parts.map(p => {
      const m = p.match(/^(\w+)(?:\s*:\s*.+)?$/);
      return m ? m[1] : null;
    }).filter(Boolean);
  }

  // Extract components from function header, handling nested parens
  function parseFnHeader(line) {
    const nm = line.match(/^fn\s+(\w+)\s*\(/);
    if (!nm) return null;
    const name = nm[1];
    let i = line.indexOf('(', nm[0].length - 1);
    const pStart = i + 1;
    let depth = 0;
    for (; i < line.length; i++) {
      if (line[i] === '(') depth++;
      else if (line[i] === ')') { depth--; if (depth === 0) break; }
    }
    const paramStr = line.slice(pStart, i);
    const rest = line.slice(i + 1).trim();
    const ci = rest.indexOf(':');
    const inline = ci !== -1 ? rest.slice(ci + 1).trim() : '';
    return { name, paramStr, inline };
  }

  // joinBody: collapse indented block into a single-line expression string
  function joinBody(lines, startIndent) {
    if (!lines.length) return '';
    // If bodyLines is array of strings (inline body), join directly
    if (typeof lines[0] === 'string') return lines.join(' ');

    // Otherwise it's array of {indent, text}
    // We do a simple recursive join:
    // Lines at startIndent are separate statements (;-separated)
    // Lines at startIndent+N are continuations of the previous
    return joinBodyRec(lines, 0, startIndent).joined;
  }

  function joinBodyRec(lines, start, targetIndent) {
    let result = '';
    let i = start;

    while (i < lines.length) {
      const { indent, text } = lines[i];
      if (indent < targetIndent) break;

      if (indent === targetIndent) {
        if (result) result += ' ; ';

        if (text.endsWith(':')) {
          // Block opener - collect sub-block
          const keyword = text.slice(0, -1); // strip trailing ':'
          i++;
          const sub = joinBodyRec(lines, i, targetIndent + 2);
          i = sub.next;
          result += keyword + ': ' + sub.joined;
          // check if next line is 'else:'
          if (i < lines.length && lines[i].indent === targetIndent && lines[i].text === 'else:') {
            i++;
            const elseSub = joinBodyRec(lines, i, targetIndent + 2);
            i = elseSub.next;
            result += ' else: ' + elseSub.joined;
          }
        } else {
          result += text;
          i++;
        }
      } else {
        // indent > targetIndent: continuation of previous
        result += ' ' + text;
        i++;
      }
    }

    return { joined: result, next: i };
  }

  // ── Expression parser ────────────────────────────────────
  // Parses a flat expression string (after joinBody)
  function parseExpr(src) {
    const toks = lex(src + '\n');
    const ctx = { toks, i: 0 };
    function peek() { return ctx.toks[ctx.i]; }
    function eat(t) {
      const tok = ctx.toks[ctx.i];
      if (t && tok.t !== t) throw new Error(`Expected ${t}, got ${tok.t} ('${tok.v||tok.t}')`);
      ctx.i++;
      return tok;
    }
    function check(...ts) { return ts.includes(peek().t); }

    function parseStatements() {
      const stmts = [];
      while (!check('EOF', 'RP', 'RB')) {
        if (check('NL', 'INDENT_VAL')) { ctx.i++; continue; }
        stmts.push(parsePipe());
        // consume optional ; separator
        if (check('ASSIGN') && peek().t === 'ASSIGN') ctx.i++;
        while (check('NL', 'INDENT_VAL')) ctx.i++;
      }
      return stmts.length === 1 ? stmts[0] : { k: 'seq', stmts };
    }

    function parsePipe() {
      let left = parseLambda();
      while (check('PIPE')) {
        eat('PIPE');
        const right = parseLambda();
        left = { k: 'pipe', left, right };
      }
      return left;
    }

    function parseLambda() {
      // lambda: ID -> expr  OR  (ID, ID) -> expr
      const save = ctx.i;
      try {
        if (check('ID')) {
          const param = eat('ID').v;
          if (check('ARROW')) {
            eat('ARROW');
            const body = parseLambda();
            return { k: 'lambda', params: [param], body };
          }
          ctx.i = save;
        } else if (check('LP')) {
          eat('LP');
          const params = [];
          while (!check('RP')) {
            if (check('COMMA')) { eat('COMMA'); continue; }
            params.push(eat('ID').v);
          }
          eat('RP');
          if (check('ARROW')) {
            eat('ARROW');
            const body = parseLambda();
            return { k: 'lambda', params, body };
          }
          ctx.i = save;
        }
      } catch(e) { ctx.i = save; }
      return parseOr();
    }

    function parseOr() {
      let left = parseAnd();
      while (check('OR')) { eat('OR'); left = { k: 'binop', op: '||', left, right: parseAnd() }; }
      return left;
    }
    function parseAnd() {
      let left = parseCmp();
      while (check('AND')) { eat('AND'); left = { k: 'binop', op: '&&', left, right: parseCmp() }; }
      return left;
    }
    function parseCmp() {
      let left = parseAdd();
      if (check('EQ','NEQ','LT','GT','LTE','GTE')) {
        const op = eat().t;
        left = { k: 'binop', op, left, right: parseAdd() };
      }
      return left;
    }
    function parseAdd() {
      let left = parseMul();
      while (check('ADD','SUB')) {
        const op = eat().t;
        left = { k: 'binop', op, left, right: parseMul() };
      }
      return left;
    }
    function parseMul() {
      let left = parseUnary();
      while (check('MUL','DIV','MOD')) {
        const op = eat().t;
        left = { k: 'binop', op, left, right: parseUnary() };
      }
      return left;
    }
    function parseUnary() {
      if (check('SUB')) { eat('SUB'); return { k: 'neg', expr: parsePostfix() }; }
      if (check('NOT')) { eat('NOT'); return { k: 'not', expr: parsePostfix() }; }
      return parsePostfix();
    }
    function parsePostfix() {
      let expr = parsePrimary();
      while (true) {
        if (check('DOT')) {
          eat('DOT');
          const method = eat('ID').v;
          if (check('LP')) {
            eat('LP');
            const args = [];
            while (!check('RP')) {
              if (check('COMMA')) { eat('COMMA'); continue; }
              args.push(parseLambda());
            }
            eat('RP');
            expr = { k: 'method', obj: expr, method, args };
          } else {
            // .method without parens — treat as .method()
            expr = { k: 'method', obj: expr, method, args: [] };
          }
        } else if (check('LP')) {
          // function application
          eat('LP');
          const args = [];
          while (!check('RP')) {
            if (check('COMMA')) { eat('COMMA'); continue; }
            args.push(parseLambda());
          }
          eat('RP');
          expr = { k: 'call', fn: expr, args };
        } else if (check('LB')) {
          // index access
          eat('LB');
          const idx = parseLambda();
          eat('RB');
          expr = { k: 'index', obj: expr, idx };
        } else {
          break;
        }
      }
      return expr;
    }
    function parsePrimary() {
      const tok = peek();

      if (tok.t === 'INT') { eat(); return { k: 'int', v: tok.v }; }
      if (tok.t === 'STR') { eat(); return { k: 'str', v: tok.v }; }
      if (tok.t === 'BOOL') { eat(); return { k: 'bool', v: tok.v === 'true' }; }

      if (tok.t === 'IF') {
        eat('IF');
        const cond = parseCmp();
        eat('COLON');
        // skip NL/INDENT
        while (check('NL','INDENT_VAL')) ctx.i++;
        const then = parsePipe();
        while (check('NL','INDENT_VAL')) ctx.i++;
        eat('ELSE');
        eat('COLON');
        while (check('NL','INDENT_VAL')) ctx.i++;
        const els = parsePipe();
        return { k: 'if', cond, then, els };
      }

      if (tok.t === 'MATCH') {
        eat('MATCH');
        const subject = parsePipe();
        eat('COLON');
        while (check('NL','INDENT_VAL')) ctx.i++;
        const arms = [];
        // Parse arms separated by ;
        while (!check('EOF','RP','RB')) {
          while (check('NL','INDENT_VAL')) ctx.i++;
          if (check('EOF','RP','RB')) break;
          const pat = parsePattern();
          eat('ARROW');
          const body = parsePipe();
          arms.push({ pat, body });
          // optional ; or newline
          if (check('ASSIGN')) ctx.i++;
          while (check('NL','INDENT_VAL')) ctx.i++;
        }
        return { k: 'match', subject, arms };
      }

      if (tok.t === 'LB') {
        eat('LB');
        const items = [];
        while (!check('RB')) {
          if (check('COMMA')) { eat('COMMA'); continue; }
          items.push(parseLambda());
        }
        eat('RB');
        return { k: 'list', items };
      }

      if (tok.t === 'LP') {
        eat('LP');
        // Could be tuple or grouped expression
        const first = parseLambda();
        if (check('DOTDOTEQ')) {
          // range like (1..=5)
          eat('DOTDOTEQ');
          const end = parseLambda();
          eat('RP');
          return { k: 'range', start: first, end };
        }
        if (check('COMMA')) {
          // tuple
          const items = [first];
          while (check('COMMA')) { eat('COMMA'); items.push(parseLambda()); }
          eat('RP');
          return { k: 'tuple', items };
        }
        eat('RP');
        return first;
      }

      if (tok.t === 'ID') {
        eat();
        const name = tok.v;
        // range shorthand: name..=expr (rare, but handle)
        if (check('DOTDOTEQ')) {
          eat('DOTDOTEQ');
          const end = parseLambda();
          return { k: 'range', start: { k: 'var', name }, end };
        }
        return { k: 'var', name };
      }

      // Skip noise tokens
      if (check('NL','INDENT_VAL','COLON')) { eat(); return parsePrimary(); }

      throw new Error(`Unexpected token: ${tok.t} ('${tok.v||tok.t}')`);
    }

    function parsePattern() {
      const tok = peek();
      if (tok.t === 'ID') {
        eat();
        const name = tok.v;
        if (check('LP')) {
          eat('LP');
          const bindings = [];
          while (!check('RP')) {
            if (check('COMMA')) { eat('COMMA'); continue; }
            bindings.push(eat('ID').v);
          }
          eat('RP');
          return { k: 'ppat', name, bindings };
        }
        return { k: 'pvar', name };
      }
      if (tok.t === 'INT') { eat(); return { k: 'plit', v: tok.v }; }
      if (tok.t === 'STR') { eat(); return { k: 'plit', v: tok.v }; }
      if (tok.t === 'BOOL') { eat(); return { k: 'plit', v: tok.v === 'true' }; }
      eat(); return { k: 'pwild' };
    }

    return parseStatements();
  }

  // ── Evaluator ────────────────────────────────────────────
  function evaluate(node, env, stdout, depth) {
    if (depth > 5000) throw new Error('Stack overflow');

    switch (node.k) {
      case 'int':  return node.v;
      case 'str':  return node.v;
      case 'bool': return node.v;
      case 'list': return node.items.map(x => evaluate(x, env, stdout, depth+1));
      case 'tuple': return node.items.map(x => evaluate(x, env, stdout, depth+1));

      case 'range': {
        const lo = evaluate(node.start, env, stdout, depth+1);
        const hi = evaluate(node.end, env, stdout, depth+1);
        const arr = [];
        for (let j = lo; j <= hi; j++) arr.push(j);
        return arr;
      }

      case 'var': {
        if (!env.has(node.name)) throw new Error(`Unbound variable: ${node.name}`);
        const v = env.get(node.name);
        return v;
      }

      case 'neg': return -evaluate(node.expr, env, stdout, depth+1);
      case 'not': return !evaluate(node.expr, env, stdout, depth+1);

      case 'binop': {
        const l = evaluate(node.left, env, stdout, depth+1);
        const r = evaluate(node.right, env, stdout, depth+1);
        switch (node.op) {
          case 'ADD': return l + r;
          case 'SUB': return l - r;
          case 'MUL': return l * r;
          case 'DIV': return r === 0 ? (() => { throw new Error('Division by zero'); })() : Math.trunc(l / r);
          case 'MOD': return ((l % r) + r) % r;
          case 'EQ':  return l === r;
          case 'NEQ': return l !== r;
          case 'LT':  return l < r;
          case 'GT':  return l > r;
          case 'LTE': return l <= r;
          case 'GTE': return l >= r;
          case '&&':  return l && r;
          case '||':  return l || r;
        }
        break;
      }

      case 'if': {
        const cond = evaluate(node.cond, env, stdout, depth+1);
        return evaluate(cond ? node.then : node.els, env, stdout, depth+1);
      }

      case 'lambda': {
        return { __fn: true, params: node.params, body: node.body, env: new Map(env) };
      }

      case 'call': {
        const callee = evaluate(node.fn, env, stdout, depth+1);
        const args = node.args.map(a => evaluate(a, env, stdout, depth+1));
        return applyFn(callee, args, stdout, depth+1);
      }

      case 'method': {
        const obj = evaluate(node.obj, env, stdout, depth+1);
        const args = node.args.map(a => evaluate(a, env, stdout, depth+1));
        return applyMethod(obj, node.method, args, env, stdout, depth+1);
      }

      case 'pipe': {
        const val = evaluate(node.left, env, stdout, depth+1);
        const fn  = evaluate(node.right, env, stdout, depth+1);
        return applyFn(fn, [val], stdout, depth+1);
      }

      case 'index': {
        const obj = evaluate(node.obj, env, stdout, depth+1);
        const idx = evaluate(node.idx, env, stdout, depth+1);
        if (Array.isArray(obj)) return obj[idx];
        if (typeof obj === 'string') return obj.charCodeAt(idx);
        throw new Error(`Cannot index into ${typeof obj}`);
      }

      case 'match': {
        const subj = evaluate(node.subject, env, stdout, depth+1);
        for (const arm of node.arms) {
          const bindings = matchPattern(arm.pat, subj);
          if (bindings !== null) {
            const innerEnv = new Map(env);
            for (const [k, v] of Object.entries(bindings)) innerEnv.set(k, v);
            return evaluate(arm.body, innerEnv, stdout, depth+1);
          }
        }
        throw new Error('Non-exhaustive match');
      }

      case 'seq': {
        let last = null;
        for (const s of node.stmts) last = evaluate(s, env, stdout, depth+1);
        return last;
      }

      default:
        throw new Error(`Unknown node kind: ${node.k}`);
    }
  }

  function matchPattern(pat, val) {
    if (pat.k === 'pwild') return {};
    if (pat.k === 'plit') return pat.v === val ? {} : null;
    if (pat.k === 'pvar') {
      // Could be wildcard name or variable capture
      if (pat.name === '_') return {};
      // Check if it's an ADT constructor with no fields
      if (val && typeof val === 'object' && val.__adt && val.tag === pat.name) return {};
      // Otherwise bind
      return { [pat.name]: val };
    }
    if (pat.k === 'ppat') {
      if (val && typeof val === 'object' && val.__adt && val.tag === pat.name) {
        const bindings = {};
        for (let i = 0; i < pat.bindings.length; i++) {
          if (pat.bindings[i] !== '_') bindings[pat.bindings[i]] = val.fields[i];
        }
        return bindings;
      }
      return null;
    }
    return null;
  }

  function applyFn(fn, args, stdout, depth) {
    if (typeof fn === 'function') return fn(...args);
    if (fn && fn.__fn) {
      // Partial application: only if FEWER args than params
      if (args.length < fn.params.length) {
        const env = new Map(fn.env);
        fn.params.slice(0, args.length).forEach((p, i) => env.set(p, args[i]));
        return { __fn: true, params: fn.params.slice(args.length), body: fn.body, env };
      }
      // Full application (including zero-arg functions)
      const env = new Map(fn.env);
      fn.params.forEach((p, i) => env.set(p, args[i]));
      return evaluate(fn.body, env, stdout, depth);
    }
    throw new Error(`Not a function: ${JSON.stringify(fn)}`);
  }

  function applyMethod(obj, method, args, env, stdout, depth) {
    // List methods
    if (Array.isArray(obj)) {
      switch (method) {
        case 'map':    return obj.map(x => applyFn(args[0], [x], stdout, depth));
        case 'filter': return obj.filter(x => applyFn(args[0], [x], stdout, depth));
        case 'sum':    return obj.reduce((a, b) => a + b, 0);
        case 'join':   return obj.join(args[0] ?? '');
        case 'take':   return obj.slice(0, args[0]);
        case 'fold':   return obj.reduce((acc, x) => applyFn(args[0], [acc, x], stdout, depth), args[1]);
        default: throw new Error(`Unknown list method: ${method}`);
      }
    }
    // Iter object (unfold)
    if (obj && obj.__iter) {
      switch (method) {
        case 'take': {
          const n = args[0];
          const result = [];
          let state = obj.state;
          for (let j = 0; j < n; j++) {
            const next = applyFn(obj.step, [state], stdout, depth);
            if (!next || !next.__adt || next.tag !== 'Next') break;
            result.push(next.fields[0]);
            state = next.fields[1];
          }
          return result;
        }
        default: throw new Error(`Unknown iter method: ${method}`);
      }
    }
    // String methods
    if (typeof obj === 'string') {
      switch (method) {
        case 'split_whitespace': return obj.trim().split(/\s+/);
        default: throw new Error(`Unknown string method: ${method}`);
      }
    }
    // Plain object (e.g. console, iter, parse namespaces)
    if (obj && typeof obj === 'object') {
      const prop = obj[method];
      if (typeof prop === 'function') {
        // With no args: return as function reference (pipe target)
        // With args: call directly
        return args.length === 0 ? prop : prop(...args);
      }
      if (prop !== undefined) return prop;
      throw new Error(`Unknown property ${method} on object`);
    }
    throw new Error(`Unknown method ${method} on ${typeof obj}`);
  }

  function showValue(v) {
    if (v === null || v === undefined) return '()';
    if (typeof v === 'string') return v;
    if (typeof v === 'number') return String(v);
    if (typeof v === 'boolean') return v ? 'true' : 'false';
    if (Array.isArray(v)) return '[' + v.map(showValue).join(', ') + ']';
    if (v && v.__adt) return v.tag + (v.fields.length ? '(' + v.fields.map(showValue).join(', ') + ')' : '');
    if (v && v.__fn) return '<fn>';
    return JSON.stringify(v);
  }

  // ── Public run() ─────────────────────────────────────────
  function run(src) {
    const lines = [];
    try {
      const { fns } = preprocess(src);

      // Build global environment
      const globalEnv = new Map();

      // Builtins
      globalEnv.set('string', { __fn: true, params: ['x'], body: { k: 'var', name: '__string__' }, env: new Map() });

      const stdout = { write: s => lines.push(s) };

      // console.println builtin
      const consolePrintln = (s) => { stdout.write(String(s)); return null; };
      globalEnv.set('console', { println: consolePrintln });

      // iter.unfold builtin
      const iterUnfold = (state, step) => ({ __iter: true, state, step });
      globalEnv.set('iter', { unfold: iterUnfold });

      // parse builtins
      globalEnv.set('parse', {
        i64: s => { const n = parseInt(s, 10); if (isNaN(n)) throw new Error(`parse.i64: "${s}"`); return n; },
        bool: s => s.trim() === 'true',
      });

      // string() as a callable fn
      const stringBuiltin = v => {
        if (typeof v === 'number') return String(v);
        if (typeof v === 'boolean') return v ? 'true' : 'false';
        if (typeof v === 'string') return v;
        return showValue(v);
      };
      globalEnv.set('string', stringBuiltin);

      // ADT constructors — Ok, Err, Next, and any user types
      globalEnv.set('Ok',   v => ({ __adt: true, tag: 'Ok',   fields: [v] }));
      globalEnv.set('Err',  v => ({ __adt: true, tag: 'Err',  fields: [v] }));
      globalEnv.set('Next', (v, s) => ({ __adt: true, tag: 'Next', fields: [v, s] }));

      // Register all functions in global env
      for (const [name, fnDef] of Object.entries(fns)) {
        // Register as a user-defined ADT constructor if name starts with capital
        // (handled via pattern matching)
        const bodyExpr = parseExpr(fnDef.body);
        const capturedParams = fnDef.params;
        globalEnv.set(name, { __fn: true, params: capturedParams, body: bodyExpr, env: new Map(globalEnv) });
      }

      // Re-set so recursive functions see each other
      for (const [name, val] of globalEnv) {
        if (val && val.__fn && val.env) val.env = new Map(globalEnv);
      }

      // Run main
      if (!globalEnv.has('main')) {
        // If no main, show all fn names and their return values
        lines.push('// no main() found');
      } else {
        const mainFn = globalEnv.get('main');
        const result = applyFn(mainFn, [], stdout, 0);
        if (result !== null && result !== undefined) {
          const shown = showValue(result);
          if (shown !== '()') lines.push(shown);
        }
      }

      // Handle console.println via interception
      // (already done above via the stdout object)

    } catch (err) {
      lines.push('Error: ' + err.message);
    }
    return lines;
  }

  // Patch: intercept console.println by replacing the builtin
  // after resolving functions (so closures capture the right env)
  return { run };

})();

// ── Playground page ───────────────────────────────────────────
function renderPlayground(app) {
  document.title = 'Playground — Arukellt';

  const exampleOptions = Object.entries(EXAMPLES)
    .map(([k, ex]) => `<option value="${k}">${ex.label}</option>`)
    .join('');

  app.innerHTML = `
<div class="playground-wrap">
  <div class="playground-header">
    <h2>// playground</h2>
    <span class="dim" style="font-size:13px;font-family:var(--mono)">Arukellt v0.0.1 — interpreter subset</span>
    <div class="run-status" id="run-status">
      <span class="status-dot"></span>ready
    </div>
  </div>
  <div class="playground-body">
    <div class="editor-pane">
      <div class="editor-toolbar">
        <label for="example-select">Example:</label>
        <select id="example-select" class="example-select">${exampleOptions}</select>
        <button class="btn btn-primary btn-sm" id="run-btn" style="margin-left:auto">▶ Run</button>
        <button class="btn btn-ghost btn-sm" id="clear-btn">Clear</button>
      </div>
      <textarea id="editor" spellcheck="false" autocorrect="off" autocapitalize="off"></textarea>
    </div>
    <div class="output-pane">
      <div class="output-toolbar">
        <span class="output-label">// output</span>
      </div>
      <div id="output"><span class="dim">Press ▶ Run to execute…</span></div>
    </div>
  </div>
</div>`;

  const editor = document.getElementById('editor');
  const output = document.getElementById('output');
  const select = document.getElementById('example-select');
  const status = document.getElementById('run-status');

  // Load initial example
  function loadExample(key) {
    const ex = EXAMPLES[key];
    if (ex) editor.value = ex.code;
  }
  loadExample(select.value);

  select.addEventListener('change', () => loadExample(select.value));

  // Tab key in textarea
  editor.addEventListener('keydown', e => {
    if (e.key === 'Tab') {
      e.preventDefault();
      const s = editor.selectionStart;
      const v = editor.value;
      editor.value = v.slice(0, s) + '  ' + v.slice(editor.selectionEnd);
      editor.selectionStart = editor.selectionEnd = s + 2;
    }
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault();
      runCode();
    }
  });

  function setStatus(cls, text) {
    status.className = 'run-status ' + cls;
    status.innerHTML = `<span class="status-dot"></span>${text}`;
  }

  async function runCode() {
    const src = editor.value;
    setStatus('running', 'running…');
    output.innerHTML = '';

    // Try WASM interpreter first, fall back to JS
    let lines, ok;
    const wasm = await loadWasm();
    if (wasm) {
      const result = runWithWasm(wasm, src);
      lines = result.lines;
      ok = result.ok;
      // Update status badge to show which engine ran
      const engine = '<span style="font-size:10px;opacity:.6"> wasm</span>';
      setStatus(ok ? 'ok' : 'err', (ok ? 'ok' : 'error') + engine);
    } else {
      // JS fallback
      try {
        lines = Arukellt.run(src);
        ok = !lines.some(l => l.startsWith('Error:'));
        setStatus(ok ? 'ok' : 'err', (ok ? 'ok' : 'error') + '<span style="font-size:10px;opacity:.6"> js</span>');
      } catch(err) {
        output.innerHTML = `<span class="err">Internal error: ${escHtml(err.message)}</span>`;
        setStatus('err', 'error');
        return;
      }
    }

    if (lines.length === 0) {
      output.innerHTML = '<span class="dim">// (no output)</span>';
    } else {
      output.innerHTML = lines.map(l => {
        if (l.startsWith('Error:')) return `<span class="err">${escHtml(l)}</span>`;
        return escHtml(l);
      }).join('\n');
    }
  }

  document.getElementById('run-btn').addEventListener('click', runCode);
  document.getElementById('clear-btn').addEventListener('click', () => {
    output.innerHTML = '<span class="dim">// cleared</span>';
    setStatus('', 'ready');
  });
}

function escHtml(s) {
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
}
