#!/usr/bin/env bash
# Generate the rich stdlib reference used by `arukellt doc --html`.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="${1:-}"

if [[ -z "$OUT" ]]; then
  echo "error: doc --html requires -o <output.html>" >&2
  exit 2
fi

mkdir -p "$(dirname "$OUT")"

awk '
function esc(s) {
  gsub(/&/, "\\&amp;", s)
  gsub(/</, "\\&lt;", s)
  gsub(/>/, "\\&gt;", s)
  gsub(/"/, "\\&quot;", s)
  gsub(/\047/, "\\&#39;", s)
  return s
}

function trim(s) {
  sub(/^[ \t\r\n]+/, "", s)
  sub(/[ \t\r\n]+$/, "", s)
  return s
}

function value_string(line, s) {
  s = line
  sub(/^[^=]*=/, "", s)
  s = trim(s)
  if (s ~ /^"/) {
    sub(/^"/, "", s)
    sub(/"([ \t]*#.*)?$/, "", s)
  }
  return s
}

function value_array(line, s) {
  s = line
  sub(/^[^=]*=/, "", s)
  s = trim(s)
  gsub(/^\[/, "", s)
  gsub(/\]([ \t]*#.*)?$/, "", s)
  gsub(/"/, "", s)
  return trim(s)
}

function value_bool(line, key, pat) {
  pat = key " = true"
  return index(line, pat) > 0 ? 1 : 0
}

function last_segment(path, parts, n) {
  n = split(path, parts, /::/)
  return parts[n]
}

function module_depth(path, parts, n) {
  n = split(path, parts, /::/)
  return n - 1
}

function add_module_node(path) {
  if (!(path in module_nodes)) {
    module_nodes[path] = 1
    module_node_count += 1
    module_list[module_node_count] = path
  }
}

function remember_module(path, parts, n, i, prefix) {
  if (path == "") {
    return
  }
  n = split(path, parts, /::/)
  prefix = parts[1]
  add_module_node(prefix)
  for (i = 2; i <= n; i += 1) {
    prefix = prefix "::" parts[i]
    add_module_node(prefix)
  }
  module_leaves[path] = 1
}

function reset_fn() {
  name = ""
  kind = ""
  module = ""
  params = ""
  returns = "()"
  stability = "provisional"
  doc = ""
  t1 = 0
  t3 = 0
}

function reset_type() {
  type_name = ""
  type_module = ""
  type_stability = "provisional"
  type_doc = ""
  type_generics = ""
}

function reset_mod() {
  mod_name = ""
}

function type_is_scalar(n) {
  return n == "bool" || n == "char" || n == "i8" || n == "i16" || n == "i32" || n == "i64" || n == "u8" || n == "u16" || n == "u32" || n == "u64" || n == "f32" || n == "f64"
}

function type_is_enum(n) {
  return n == "Option" || n == "Result" || n == "Ordering"
}

function item_summary(name, meta, doc, badge, href,   safe_name, safe_meta, safe_doc, safe_badge) {
  safe_name = esc(name)
  safe_meta = esc(meta)
  safe_doc = esc(doc)
  safe_badge = esc(badge)
  if (safe_doc == "") {
    safe_doc = "No manifest documentation yet."
  }
  return "<article class=\"summary-item\"><a href=\"#" href "\">" safe_name "</a><span>" safe_badge "</span><code>" safe_meta "</code><p>" safe_doc "</p></article>"
}

function flush_fn(   safe_name, safe_module, safe_stability, sig, safe_sig, safe_doc, target, search) {
  if (section != "fn" || name == "" || kind == "intrinsic" || module == "") {
    return
  }
  remember_module(module)
  fn_count += 1
  safe_name = esc(name)
  safe_module = esc(module)
  safe_stability = esc(stability)
  sig = "fn " name "(" params ") -> " returns
  safe_sig = esc(sig)
  safe_doc = esc(doc)
  if (safe_doc == "") {
    safe_doc = "No manifest documentation yet."
  }
  if (t1 && t3) {
    target = "T1 + T3"
  } else if (t3) {
    target = "T3"
  } else if (t1) {
    target = "T1"
  } else {
    target = "target-gated"
  }
  search = esc(name " " module " " sig " " stability)
  function_cards[fn_count] = "<article class=\"api-card\" data-module=\"" safe_module "\" data-stability=\"" safe_stability "\" data-search=\"" search "\"><div class=\"api-card-head\"><a class=\"api-name\" id=\"fn-" safe_name "\" href=\"#fn-" safe_name "\">" safe_name "</a><span class=\"badge stability\">" safe_stability "</span><span class=\"badge target\">" esc(target) "</span></div><div class=\"module\">" safe_module "</div><pre><code>" safe_sig "</code></pre><p>" safe_doc "</p></article>"
}

function flush_type(   meta, href, card) {
  if (section != "type" || type_name == "" || type_is_scalar(type_name)) {
    return
  }
  if (type_module != "") {
    remember_module(type_module)
  }
  meta = type_module
  if (meta == "") {
    meta = "prelude"
  }
  if (type_generics != "") {
    meta = meta " <" type_generics ">"
  }
  href = "type-" type_name
  if (type_is_enum(type_name)) {
    enum_count += 1
    enum_cards[enum_count] = item_summary(type_name, meta, type_doc, type_stability, href)
  } else {
    struct_count += 1
    struct_cards[struct_count] = item_summary(type_name, meta, type_doc, type_stability, href)
  }
}

function flush_mod() {
  if (section == "mod" && mod_name != "") {
    remember_module(mod_name)
  }
}

function flush_current() {
  flush_fn()
  flush_type()
  flush_mod()
}

function print_module_tree(   n, sorted, i, j, tmp, m, depth, label, leaf, cls) {
  n = module_node_count
  for (i = 1; i <= n; i += 1) {
    sorted[i] = module_list[i]
  }
  for (i = 2; i <= n; i += 1) {
    tmp = sorted[i]
    j = i - 1
    while (j >= 1 && sorted[j] > tmp) {
      sorted[j + 1] = sorted[j]
      j -= 1
    }
    sorted[j + 1] = tmp
  }
  print "<div class=\"module-tree\">"
  for (i = 1; i <= n; i += 1) {
    m = sorted[i]
    depth = module_depth(m)
    label = last_segment(m)
    leaf = module_leaves[m] ? " leaf" : ""
    cls = "module-node depth-" depth leaf
    print "<a class=\"" cls "\" href=\"#modules\" data-module-filter=\"" esc(m) "\" title=\"" esc(m) "\"><span>" esc(label) "</span></a>"
  }
  print "</div>"
}

function print_empty(label) {
  print "<p class=\"empty\">No " label " are recorded in std/manifest.toml yet.</p>"
}

BEGIN {
  section = ""
  reset_fn()
  reset_type()
  reset_mod()
}

/^\[\[functions\]\]/ {
  flush_current()
  section = "fn"
  reset_fn()
  next
}

/^\[\[types\]\]/ {
  flush_current()
  section = "type"
  reset_type()
  next
}

/^\[\[modules\]\]/ {
  flush_current()
  section = "mod"
  reset_mod()
  next
}

section == "fn" && /^name = / { name = value_string($0); next }
section == "fn" && /^kind = / { kind = value_string($0); next }
section == "fn" && /^module = / { module = value_string($0); next }
section == "fn" && /^params = / { params = value_array($0); next }
section == "fn" && /^returns = / { returns = value_string($0); next }
section == "fn" && /^stability = / { stability = value_string($0); next }
section == "fn" && /^doc = / { doc = value_string($0); next }
section == "fn" && /^availability = / { t1 = value_bool($0, "t1"); t3 = value_bool($0, "t3"); next }

section == "type" && /^name = / { type_name = value_string($0); next }
section == "type" && /^module = / { type_module = value_string($0); next }
section == "type" && /^stability = / { type_stability = value_string($0); next }
section == "type" && /^doc = / { type_doc = value_string($0); next }
section == "type" && /^generic_params = / { type_generics = value_array($0); next }

section == "mod" && /^name = / { mod_name = value_string($0); next }

END {
  flush_current()
  module_count = length(module_leaves)

  print "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Arukellt std docs</title><style>"
  print ":root{color-scheme:light;--bg:#f7f8fb;--panel:#fff;--ink:#17202a;--muted:#5c6675;--line:#d8dee8;--accent:#0b6bcb;--ok:#1f7a4d;--warn:#9a6700}*{box-sizing:border-box}body{margin:0;font:15px/1.55 system-ui,-apple-system,Segoe UI,sans-serif;background:var(--bg);color:var(--ink)}header{background:#111827;color:white;padding:28px 32px}header h1{margin:0 0 8px;font-size:32px;letter-spacing:0}header p{margin:0;color:#d1d5db;max-width:880px}.layout{display:grid;grid-template-columns:300px 1fr;gap:24px;padding:24px}.sidebar{position:sticky;top:16px;align-self:start;background:var(--panel);border:1px solid var(--line);padding:16px;max-height:calc(100vh - 32px);overflow:auto}.sidebar h2{font-size:15px;margin:0 0 8px}.sidebar a{color:var(--accent);text-decoration:none}.module-tree{display:grid;gap:2px}.module-node{display:block;padding:3px 0;color:var(--muted)}.module-node.leaf{color:var(--accent)}.module-node span{display:inline-block}.depth-1 span{margin-left:14px}.depth-2 span{margin-left:28px}.depth-3 span{margin-left:42px}.depth-4 span{margin-left:56px}.depth-5 span{margin-left:70px}.toolbar{display:grid;grid-template-columns:1fr 180px;gap:12px;margin-bottom:16px}.toolbar input,.toolbar select{width:100%;padding:10px;border:1px solid var(--line);background:white}.stats{display:flex;gap:12px;flex-wrap:wrap;margin:16px 0}.stat{background:white;border:1px solid var(--line);padding:10px 12px}.item-section{margin:0 0 28px}.item-section h2{font-size:24px;margin:0 0 10px}.summary-grid,.api-grid{display:grid;gap:12px}.summary-item,.api-card{background:var(--panel);border:1px solid var(--line);padding:16px}.summary-item a,.api-name{font-size:19px;font-weight:700;color:var(--accent);text-decoration:none}.summary-item span{border:1px solid var(--line);padding:2px 7px;font-size:12px;margin-left:8px;color:var(--ok)}.summary-item code{display:block;color:var(--muted);margin-top:6px}.summary-item p,.api-card p{margin-bottom:0}.api-card-head{display:flex;gap:8px;align-items:center;flex-wrap:wrap}.module{color:var(--muted);font-family:ui-monospace,monospace;margin:6px 0}.badge{border:1px solid var(--line);padding:2px 7px;font-size:12px}.stability{color:var(--ok)}.target{color:var(--warn)}.empty{background:var(--panel);border:1px solid var(--line);color:var(--muted);padding:14px}pre{overflow:auto;background:#f3f5f8;border:1px solid var(--line);padding:10px}code{font-family:ui-monospace,SFMono-Regular,Menlo,monospace}@media(max-width:860px){.layout{grid-template-columns:1fr}.sidebar{position:static;max-height:none}.toolbar{grid-template-columns:1fr}}</style></head><body>"
  print "<header><h1>Arukellt Standard Library</h1><p>Manifest-backed API reference generated by <code>arukellt doc --html</code>. Search by function, module, signature, or stability.</p></header><main class=\"layout\"><aside class=\"sidebar\"><h2>Modules</h2><a class=\"module-node leaf\" href=\"#modules\" data-module-filter=\"\"><span>All modules</span></a>"
  print_module_tree()
  print "</aside><section>"
  print "<div class=\"stats\"><div class=\"stat\"><strong>" module_count "</strong> modules</div><div class=\"stat\"><strong>0</strong> macros</div><div class=\"stat\"><strong>" struct_count "</strong> structs</div><div class=\"stat\"><strong>" enum_count "</strong> enums</div><div class=\"stat\"><strong>" fn_count "</strong> functions</div><div class=\"stat\"><strong>0</strong> type aliases</div></div>"

  print "<section id=\"modules\" class=\"item-section\"><h2>Modules</h2>"
  print_module_tree()
  print "</section>"

  print "<section id=\"macros\" class=\"item-section\"><h2>Macros</h2>"
  print_empty("macros")
  print "</section>"

  print "<section id=\"structs\" class=\"item-section\"><h2>Structs</h2><div class=\"summary-grid\">"
  if (struct_count == 0) {
    print_empty("structs")
  } else {
    for (i = 1; i <= struct_count; i += 1) print struct_cards[i]
  }
  print "</div></section>"

  print "<section id=\"enums\" class=\"item-section\"><h2>Enums</h2><div class=\"summary-grid\">"
  if (enum_count == 0) {
    print_empty("enums")
  } else {
    for (i = 1; i <= enum_count; i += 1) print enum_cards[i]
  }
  print "</div></section>"

  print "<section id=\"functions\" class=\"item-section\"><h2>Functions</h2><div class=\"toolbar\"><input id=\"q\" placeholder=\"Search functions\"><select id=\"stability\"><option value=\"\">All stability tiers</option><option>stable</option><option>provisional</option><option>experimental</option><option>deprecated</option></select></div><div id=\"api\" class=\"api-grid\">"
  for (i = 1; i <= fn_count; i += 1) print function_cards[i]
  print "</div></section>"

  print "<section id=\"type-aliases\" class=\"item-section\"><h2>Type Aliases</h2>"
  print_empty("type aliases")
  print "</section>"

  print "</section></main><script>const q=document.getElementById(\"q\");const st=document.getElementById(\"stability\");let mod=\"\";function moduleMatch(value){return !mod||value===mod||value.startsWith(mod+\"::\");}function apply(){const needle=(q.value||\"\").toLowerCase();document.querySelectorAll(\".api-card\").forEach(c=>{const okText=!needle||(c.dataset.search||\"\").toLowerCase().includes(needle);const okSt=!st.value||c.dataset.stability===st.value;const okMod=moduleMatch(c.dataset.module||\"\");c.style.display=okText&&okSt&&okMod?\"block\":\"none\";});}document.querySelectorAll(\"[data-module-filter]\").forEach(a=>a.addEventListener(\"click\",e=>{e.preventDefault();mod=a.dataset.moduleFilter||\"\";apply();document.getElementById(\"functions\").scrollIntoView();}));q.addEventListener(\"input\",apply);st.addEventListener(\"change\",apply);</script></body></html>"
}
' "$ROOT/std/manifest.toml" > "$OUT"

echo "generated std docs: $OUT"
