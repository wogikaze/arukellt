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

function flush_fn(   safe_name, safe_module, safe_stability, sig, safe_sig, safe_doc, target, search) {
  if (section != "fn" || name == "" || kind == "intrinsic" || module == "") {
    return
  }
  fn_count += 1
  modules[module] = 1
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
  cards[fn_count] = "<article class=\"api-card\" data-module=\"" safe_module "\" data-stability=\"" safe_stability "\" data-search=\"" search "\"><div class=\"api-card-head\"><a class=\"api-name\" id=\"" safe_name "\" href=\"#" safe_name "\">" safe_name "</a><span class=\"badge stability\">" safe_stability "</span><span class=\"badge target\">" esc(target) "</span></div><div class=\"module\">" safe_module "</div><pre><code>" safe_sig "</code></pre><p>" safe_doc "</p></article>"
}

function reset_mod() {
  mod_name = ""
}

function flush_mod() {
  if (section == "mod" && mod_name != "") {
    modules[mod_name] = 1
  }
}

BEGIN {
  section = ""
  reset_fn()
  reset_mod()
}

/^\[\[functions\]\]/ {
  flush_fn()
  flush_mod()
  section = "fn"
  reset_fn()
  next
}

/^\[\[modules\]\]/ {
  flush_fn()
  flush_mod()
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
section == "mod" && /^name = / { mod_name = value_string($0); next }

END {
  flush_fn()
  flush_mod()
  module_count = 0
  for (m in modules) {
    module_count += 1
    module_links[module_count] = m
  }

  print "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Arukellt std docs</title><style>"
  print ":root{color-scheme:light;--bg:#f7f8fb;--panel:#fff;--ink:#17202a;--muted:#5c6675;--line:#d8dee8;--accent:#0b6bcb;--ok:#1f7a4d;--warn:#9a6700}*{box-sizing:border-box}body{margin:0;font:15px/1.55 system-ui,-apple-system,Segoe UI,sans-serif;background:var(--bg);color:var(--ink)}header{background:#111827;color:white;padding:28px 32px}header h1{margin:0 0 8px;font-size:32px;letter-spacing:0}header p{margin:0;color:#d1d5db;max-width:880px}.layout{display:grid;grid-template-columns:280px 1fr;gap:24px;padding:24px}.sidebar{position:sticky;top:16px;align-self:start;background:var(--panel);border:1px solid var(--line);padding:16px}.sidebar a{display:block;color:var(--accent);text-decoration:none;padding:4px 0}.toolbar{display:grid;grid-template-columns:1fr 180px;gap:12px;margin-bottom:16px}.toolbar input,.toolbar select{width:100%;padding:10px;border:1px solid var(--line);background:white}.stats{display:flex;gap:12px;flex-wrap:wrap;margin:16px 0}.stat{background:white;border:1px solid var(--line);padding:10px 12px}.api-grid{display:grid;gap:12px}.api-card{background:var(--panel);border:1px solid var(--line);padding:16px}.api-card-head{display:flex;gap:8px;align-items:center;flex-wrap:wrap}.api-name{font-size:19px;font-weight:700;color:var(--accent);text-decoration:none}.module{color:var(--muted);font-family:ui-monospace,monospace;margin:6px 0}.badge{border:1px solid var(--line);padding:2px 7px;font-size:12px}.stability{color:var(--ok)}.target{color:var(--warn)}pre{overflow:auto;background:#f3f5f8;border:1px solid var(--line);padding:10px}code{font-family:ui-monospace,SFMono-Regular,Menlo,monospace}@media(max-width:860px){.layout{grid-template-columns:1fr}.sidebar{position:static}.toolbar{grid-template-columns:1fr}}</style></head><body>"
  print "<header><h1>Arukellt Standard Library</h1><p>Manifest-backed API reference generated by <code>arukellt doc --html</code>. Search by function, module, signature, or stability.</p></header><main class=\"layout\"><aside class=\"sidebar\"><strong>Modules</strong><a href=\"#\" data-module-filter=\"\">All modules</a>"
  for (i = 1; i <= module_count; i += 1) {
    m = esc(module_links[i])
    print "<a href=\"#\" data-module-filter=\"" m "\">" m "</a>"
  }
  print "</aside><section><div class=\"toolbar\"><input id=\"q\" placeholder=\"Search std APIs\"><select id=\"stability\"><option value=\"\">All stability tiers</option><option>stable</option><option>provisional</option><option>experimental</option><option>deprecated</option></select></div><div class=\"stats\"><div class=\"stat\"><strong>" fn_count "</strong> functions</div><div class=\"stat\"><strong>" module_count "</strong> modules</div></div><div id=\"api\" class=\"api-grid\">"
  for (i = 1; i <= fn_count; i += 1) {
    print cards[i]
  }
  print "</div></section></main><script>const q=document.getElementById(\"q\");const st=document.getElementById(\"stability\");let mod=\"\";function apply(){const needle=(q.value||\"\").toLowerCase();document.querySelectorAll(\".api-card\").forEach(c=>{const okText=!needle||(c.dataset.search||\"\").toLowerCase().includes(needle);const okSt=!st.value||c.dataset.stability===st.value;const okMod=!mod||c.dataset.module===mod;c.style.display=okText&&okSt&&okMod?\"block\":\"none\";});}document.querySelectorAll(\"[data-module-filter]\").forEach(a=>a.addEventListener(\"click\",e=>{e.preventDefault();mod=a.dataset.moduleFilter||\"\";apply();}));q.addEventListener(\"input\",apply);st.addEventListener(\"change\",apply);</script></body></html>"
}
' "$ROOT/std/manifest.toml" > "$OUT"

echo "generated std docs: $OUT"
