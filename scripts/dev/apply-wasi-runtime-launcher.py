#!/usr/bin/env python3
"""Temporary launcher migration for #841/#819; removed before PR readiness."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PATH = ROOT / "scripts/run/arukellt-selfhost.sh"

text = PATH.read_text(encoding="utf-8")

if "plug_runtime_adapter_in_place()" not in text:
    anchor = '''}

resolve_selfhost_wasm() {'''
    helper = '''}

runtime_adapter_path() {
  echo "$REPO_ROOT/runtime/wasi-p2-adapter/runtime-adapter.wasm"
}

component_needs_runtime_adapter() {
  local component="$1"
  grep -aqF 'runtime-http-get' "$component" 2>/dev/null \
    || grep -aqF 'runtime-fs-read-file' "$component" 2>/dev/null \
    || grep -aqF 'runtime-env-vars' "$component" 2>/dev/null
}

plug_runtime_adapter_in_place() {
  local component="$1"
  if ! component_needs_runtime_adapter "$component"; then
    return 0
  fi
  local adapter tmp
  adapter="$(runtime_adapter_path)"
  if [[ ! -f "$adapter" ]]; then
    echo "arukellt-selfhost: error — checked WASI runtime adapter missing: $adapter" >&2
    return 1
  fi
  if ! command -v wac >/dev/null 2>&1; then
    echo "arukellt-selfhost: error — wac is required to link the WASI runtime adapter" >&2
    return 127
  fi
  tmp="${component}.runtime-link.tmp.wasm"
  rm -f "$tmp"
  wac plug --plug "$adapter" "$component" -o "$tmp"
  mv "$tmp" "$component"
}

resolve_selfhost_wasm() {'''
    if anchor not in text:
        raise SystemExit("launcher helper anchor missing")
    text = text.replace(anchor, helper, 1)

if "# #819/#841: direct component compile" not in text:
    anchor = '''if [[ "${1:-}" == "run" ]]; then
'''
    block = '''# #819/#841: direct component compile is a two-stage product operation:
# selfhost emits the raw command component, then the checked real-WASI adapter
# is plugged into its versioned runtime imports. The resulting file has no
# arukellt-host-run dependency and is directly runnable by stock Wasmtime.
if [[ "${1:-}" == "compile" ]]; then
  component_emit=0
  i=1
  while [[ $i -le $# ]]; do
    arg="${!i}"
    if [[ "$arg" == "--emit=component" ]]; then
      component_emit=1
    elif [[ "$arg" == "--emit" ]]; then
      next=$((i + 1))
      if [[ $next -le $# && "${!next}" == "component" ]]; then
        component_emit=1
      fi
    fi
    i=$((i + 1))
  done
  if [[ "$component_emit" -eq 1 ]]; then
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT
    set +e
    run_compiler "$wasm" "$@" >"$tmpdir/stdout" 2>"$tmpdir/stderr"
    rc=$?
    set -e
    cat "$tmpdir/stdout"
    if [[ "$rc" -ne 0 ]]; then
      cat "$tmpdir/stderr" >&2
      exit "$rc"
    fi
    out_path="$(sed -n 's/^wrote .* to //p' "$tmpdir/stderr" | tail -n 1)"
    if [[ -z "$out_path" ]]; then
      out_path="$(sed -n 's/^compiled .* -> //p' "$tmpdir/stderr" | tail -n 1)"
    fi
    if [[ -z "$out_path" ]]; then
      cat "$tmpdir/stderr" >&2
      echo "arukellt-selfhost: error — component compile produced no output path" >&2
      exit 1
    fi
    if [[ "$out_path" != /* ]]; then
      out_path="$REPO_ROOT/$out_path"
    fi
    plug_runtime_adapter_in_place "$out_path"
    cat "$tmpdir/stderr" >&2
    exit 0
  fi
fi

if [[ "${1:-}" == "run" ]]; then
'''
    if anchor not in text:
        raise SystemExit("run block anchor missing")
    text = text.replace(anchor, block, 1)

old_exec = '''    exec wasmtime run --wasm gc --wasm function-references --dir="$REPO_ROOT" "$out_path"'''
new_exec = '''    plug_runtime_adapter_in_place "$out_path"
    exec wasmtime run --wasm gc --wasm function-references --dir="$REPO_ROOT" "$out_path"'''
if old_exec in text:
    text = text.replace(old_exec, new_exec, 1)

old_component = '''  if [[ "$wasm_ver" == "13" ]]; then
    exec wasmtime run --wasm gc --wasm function-references --dir="$REPO_ROOT" "$out_path"
  fi'''
new_component = '''  if [[ "$wasm_ver" == "13" ]]; then
    plug_runtime_adapter_in_place "$out_path"
    exec wasmtime run --wasm gc --wasm function-references --dir="$REPO_ROOT" "$out_path"
  fi'''
if old_component in text:
    text = text.replace(old_component, new_component, 1)

old_hosted = '''  # Hosted runner for simplified guest ABI on WIT-shaped modules (#727 Phase 2/3).
  # Real WASI method names (handle / start-connect / …) go through wasmtime once Phase 4 lands.
  if grep -aqE 'http_get|http_request|http_serve|sockets_connect|sockets_read|sockets_write|sockets_listen|sockets_accept' "$out_path" 2>/dev/null; then
    exec "$REPO_ROOT/scripts/run/arukellt-run-hosted.sh" --dir="$REPO_ROOT" "$out_path"
  fi
'''
text = text.replace(old_hosted, "")

# Earlier write-back attempts could reapply `old_exec` on every run because
# the replacement still contains that exact exec line. Collapse all adjacent
# duplicate adapter-link calls after edits so this migration is idempotent.
text = re.sub(
    r'(?m)^(\s*)plug_runtime_adapter_in_place "\$out_path"\n(?:\1plug_runtime_adapter_in_place "\$out_path"\n)+',
    lambda m: f'{m.group(1)}plug_runtime_adapter_in_place "$out_path"\n',
    text,
)
PATH.write_text(text, encoding="utf-8")

deny_script = ROOT / "scripts/dev/apply-deny-process.py"
if deny_script.exists():
    code = compile(deny_script.read_text(encoding="utf-8"), str(deny_script), "exec")
    exec(code, {"__name__": "__main__", "__file__": str(deny_script)})
