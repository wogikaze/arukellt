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
  return "<div class=\"item-row\"><dt><a id=\"" href "\" href=\"#" href "\">" safe_name "</a><span class=\"stab\">" safe_badge "</span></dt><dd><code>" safe_meta "</code><p>" safe_doc "</p></dd></div>"
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
  function_cards[fn_count] = "<div class=\"item-row api-card\" data-module=\"" safe_module "\" data-stability=\"" safe_stability "\" data-search=\"" search "\"><dt><a id=\"fn-" safe_module "::" safe_name "\" href=\"#fn-" safe_module "::" safe_name "\">" safe_name "</a><span class=\"stab\">" safe_stability "</span><span class=\"target\">" esc(target) "</span></dt><dd><div class=\"module\">" safe_module "</div><pre><code>" safe_sig "</code></pre><p>" safe_doc "</p></dd></div>"
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

function print_module_tree(   n, sorted, i, j, tmp, m, depth, label, leaf, cls, parent, expanded, hidden) {
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
    parent = m
    sub(/::[^:]+$/, "", parent)
    if (parent == m) {
      parent = ""
    }
    expanded = ""
    hidden = depth > 0 ? " hidden" : ""
    cls = "module-node depth-" depth leaf expanded hidden
    print "<button type=\"button\" class=\"" cls "\" data-module=\"" esc(m) "\" data-parent=\"" esc(parent) "\" data-module-filter=\"" esc(m) "\" title=\"" esc(m) "\"><span class=\"twisty\"></span><span class=\"label\">" esc(label) "</span></button>"
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

/^\[\[values\]\]/ {
  flush_current()
  section = "value"
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
  print ":root{color-scheme:light;--bg:#fff;--sidebar:#f6f7f9;--ink:#1f2933;--muted:#68707c;--line:#d8dee8;--accent:#3873b3;--code:#f5f5f5;--ok:#2e7d32;--warn:#8a5a00}*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--ink);font:14px/1.5 system-ui,-apple-system,Segoe UI,sans-serif}a{color:var(--accent);text-decoration:none}a:hover{text-decoration:underline}.topbar{border-bottom:1px solid var(--line);background:#fafafa;padding:8px 18px;display:flex;gap:18px;align-items:center}.topbar strong{font-size:15px}.topbar a{color:var(--muted)}.layout{display:grid;grid-template-columns:300px minmax(0,1fr);min-height:calc(100vh - 37px)}.sidebar{background:var(--sidebar);border-right:1px solid var(--line);padding:14px 12px;overflow:auto;position:sticky;top:0;height:calc(100vh - 37px)}.sidebar h2{font-size:13px;margin:0 0 8px;color:#333}.module-tree{display:block}.module-node{appearance:none;border:0;background:transparent;width:100%;display:flex;align-items:center;gap:4px;padding:2px 4px;text-align:left;color:var(--muted);font:13px/1.35 ui-monospace,SFMono-Regular,Menlo,monospace;cursor:pointer}.module-node:hover{background:#e9edf3;color:var(--accent)}.module-node.leaf{color:var(--accent)}.module-node.hidden{display:none}.twisty{width:11px;color:#777}.module-node:not(.leaf) .twisty::before{content:\"▸\"}.module-node.expanded:not(.leaf) .twisty::before{content:\"▾\"}.module-node.leaf .twisty::before{content:\"\"}.depth-1{padding-left:18px}.depth-2{padding-left:36px}.depth-3{padding-left:54px}.depth-4{padding-left:72px}.depth-5{padding-left:90px}.content{padding:24px 36px;max-width:980px}.crumb{color:var(--muted);margin:0 0 12px}.title{display:flex;align-items:baseline;gap:10px;margin:0 0 8px}.title h1{font-size:32px;font-weight:600;margin:0;letter-spacing:0}.copy-path{color:var(--muted);font-size:13px}.summary{border-left:4px solid var(--line);padding-left:14px;color:#39414d;max-width:820px}.stats{display:flex;gap:8px;flex-wrap:wrap;margin:18px 0}.stat{border:1px solid var(--line);background:#fafafa;border-radius:3px;padding:4px 8px;color:var(--muted)}.stat strong{color:var(--ink)}.item-section{margin:28px 0}.item-section h2{font-size:22px;font-weight:600;margin:0 0 10px;border-bottom:1px solid var(--line);padding-bottom:6px}.item-list{display:grid;gap:0}.item-row{display:grid;grid-template-columns:230px minmax(0,1fr);border-bottom:1px solid #edf0f3;padding:8px 0}.item-row dt{margin:0;font-weight:600}.item-row dd{margin:0;color:#303946}.item-row p{margin:4px 0 0;color:#4f5965}.stab,.target{display:inline-block;margin-left:7px;color:var(--muted);font-size:12px;font-weight:400}.stab{color:var(--ok)}.target{color:var(--warn)}.module{color:var(--muted);font-family:ui-monospace,SFMono-Regular,Menlo,monospace;margin-bottom:3px}.toolbar{display:grid;grid-template-columns:1fr 170px;gap:10px;margin-bottom:10px}.toolbar input,.toolbar select{width:100%;padding:7px 8px;border:1px solid var(--line);background:#fff;border-radius:3px}.empty{color:var(--muted);margin:0;padding:7px 0}pre{margin:4px 0 0;overflow:auto;background:var(--code);border:1px solid #e2e5e9;border-radius:3px;padding:7px 8px}code{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:13px}@media(max-width:860px){.topbar{display:block}.layout{display:block}.sidebar{position:static;height:auto;max-height:320px;border-right:0;border-bottom:1px solid var(--line)}.content{padding:20px}.item-row{grid-template-columns:1fr;gap:2px}.toolbar{grid-template-columns:1fr}}</style></head><body>"
  print "<nav class=\"topbar\"><strong>Arukellt std</strong><a href=\"#functions\">Search</a><a href=\"#structs\">Structs</a><a href=\"#enums\">Enums</a><a href=\"#type-aliases\">Type Aliases</a></nav><main class=\"layout\"><aside class=\"sidebar\"><h2>Modules</h2><button type=\"button\" class=\"module-node leaf\" data-module-filter=\"\"><span class=\"twisty\"></span><span class=\"label\">All modules</span></button>"
  print_module_tree()
  print "</aside><section class=\"content\"><p class=\"crumb\">std</p><div class=\"title\"><h1>Crate std</h1><span class=\"copy-path\">Copy item path</span></div><p class=\"summary\">Manifest-backed API reference generated by <code>arukellt doc --html</code>. Search by function, module, signature, or stability.</p>"
  print "<div class=\"stats\"><div class=\"stat\"><strong>" module_count "</strong> modules</div><div class=\"stat\"><strong>0</strong> macros</div><div class=\"stat\"><strong>" struct_count "</strong> structs</div><div class=\"stat\"><strong>" enum_count "</strong> enums</div><div class=\"stat\"><strong>" fn_count "</strong> functions</div><div class=\"stat\"><strong>0</strong> type aliases</div></div>"

  print "<section id=\"macros\" class=\"item-section\"><h2>Macros</h2>"
  print_empty("macros")
  print "</section>"

  print "<section id=\"structs\" class=\"item-section\"><h2>Structs</h2><dl class=\"item-list\">"
  if (struct_count == 0) {
    print_empty("structs")
  } else {
    for (i = 1; i <= struct_count; i += 1) print struct_cards[i]
  }
  print "</dl></section>"

  print "<section id=\"enums\" class=\"item-section\"><h2>Enums</h2><dl class=\"item-list\">"
  if (enum_count == 0) {
    print_empty("enums")
  } else {
    for (i = 1; i <= enum_count; i += 1) print enum_cards[i]
  }
  print "</dl></section>"

  print "<section id=\"functions\" class=\"item-section\"><h2>Functions</h2><div class=\"toolbar\"><input id=\"q\" placeholder=\"Search functions\"><select id=\"stability\"><option value=\"\">All stability tiers</option><option>stable</option><option>provisional</option><option>experimental</option><option>deprecated</option></select></div><dl id=\"api\" class=\"item-list\">"
  for (i = 1; i <= fn_count; i += 1) print function_cards[i]
  print "</dl></section>"

  print "<section id=\"type-aliases\" class=\"item-section\"><h2>Type Aliases</h2>"
  print_empty("type aliases")
  print "</section>"

  print "</section></main><script>const q=document.getElementById(\"q\");const st=document.getElementById(\"stability\");let mod=\"\";function moduleMatch(value){return !mod||value===mod||value.startsWith(mod+\"::\");}function apply(){const needle=(q.value||\"\").toLowerCase();document.querySelectorAll(\".api-card\").forEach(c=>{const okText=!needle||(c.dataset.search||\"\").toLowerCase().includes(needle);const okSt=!st.value||c.dataset.stability===st.value;const okMod=moduleMatch(c.dataset.module||\"\");c.style.display=okText&&okSt&&okMod?\"grid\":\"none\";});}function childrenOf(path){return Array.from(document.querySelectorAll(\".module-node[data-parent=\\\"\"+path+\"\\\"]\"));}function hideDesc(path){childrenOf(path).forEach(c=>{c.classList.add(\"hidden\");c.classList.remove(\"expanded\");hideDesc(c.dataset.module||\"\");});}document.querySelectorAll(\".module-node\").forEach(a=>a.addEventListener(\"click\",e=>{e.preventDefault();const path=a.dataset.module||\"\";if(!a.classList.contains(\"leaf\")){const open=!a.classList.contains(\"expanded\");a.classList.toggle(\"expanded\",open);childrenOf(path).forEach(c=>c.classList.toggle(\"hidden\",!open));if(!open)hideDesc(path);}mod=a.dataset.moduleFilter||\"\";apply();}));q.addEventListener(\"input\",apply);st.addEventListener(\"change\",apply);</script></body></html>"
}
' "$ROOT/std/manifest.toml" > "$OUT"

echo "generated std docs: $OUT"
