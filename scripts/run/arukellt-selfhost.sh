#!/usr/bin/env bash
# scripts/run/arukellt-selfhost.sh — Selfhost-only arukellt entrypoint (#559, #583).
#
# This wrapper executes the **selfhost wasm** for every invocation. Per #583
# (and ADR-029, #585) the legacy `ARUKELLT_USE_RUST=1` opt-in has been
# RETIRED — the Rust legacy CLI no longer exists. Setting `ARUKELLT_USE_RUST`
# now hard-fails with a pointer to this notice.
#
# Resolution order:
#   1. `$ARUKELLT_SELFHOST_WASM`
#   2. `.build/selfhost/arukellt-s3.wasm` (stage-3 self-compile)
#   3. `.build/selfhost/arukellt-s2.wasm`
#   4. `.bootstrap-build/arukellt-s2.wasm`
#   4. `bootstrap/arukellt-selfhost.wasm` (committed pinned reference;
#      see `bootstrap/PROVENANCE.md`)
# Then `wasmtime run --dir=<repo_root> <wasm> -- "$@"`.
#
# Exit codes are forwarded from the underlying selfhost process.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

# Match scripts/selfhost/checks.py WASMTIME_SELFHOST_WASM_FLAGS so Memory64
# s2-runtime modules can grow past the wasm32 4GiB ceiling.
WASMTIME_SELFHOST_FLAGS=(
  --wasm gc
  --wasm function-references
  -W memory64=y
  -W max-memory-size=17179869184
)

is_truthy() {
  case "${1:-}" in
    1|true|TRUE|True|yes|YES|on|ON) return 0 ;;
    *) return 1 ;;
  esac
}

# memory32 wasm32-gc / wasi-p2 compilers import wasi:cli/* (host-linker; #834).
compiler_needs_host_linker() {
  local wasm_path="$1"
  grep -aqF 'wasi:cli/' "$wasm_path" 2>/dev/null \
    || grep -aqF 'wasi:filesystem/' "$wasm_path" 2>/dev/null \
    || grep -aqF 'arukellt:fs@' "$wasm_path" 2>/dev/null
}

run_compiler() {
  local wasm_path="$1"
  shift
  if compiler_needs_host_linker "$wasm_path"; then
    "$REPO_ROOT/scripts/run/arukellt-run-hosted.sh" --dir="$REPO_ROOT" "$wasm_path" -- "$@"
    return $?
  fi
  wasmtime run "${WASMTIME_SELFHOST_FLAGS[@]}" --dir="$REPO_ROOT" "$wasm_path" -- "$@"
}

runtime_adapter_path() {
  echo "$REPO_ROOT/runtime/wasi-p2-adapter/runtime-adapter.wasm"
}

component_needs_runtime_adapter() {
  local component="$1"
  grep -aqF 'runtime-http-get' "$component" 2>/dev/null     || grep -aqF 'runtime-fs-read-file' "$component" 2>/dev/null     || grep -aqF 'runtime-env-vars' "$component" 2>/dev/null
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

resolve_selfhost_wasm() {
  if [[ -n "${ARUKELLT_SELFHOST_WASM:-}" ]] && [[ -f "$ARUKELLT_SELFHOST_WASM" ]]; then
    echo "$ARUKELLT_SELFHOST_WASM"; return 0
  fi
  # Prefer the heap-patched s2-runtime wasm (4 GiB linear memory) over the
  # unpatched s3 wasm (512 MiB).  The s3 wasm is the fixpoint self-recompile
  # output but lacks the heap-grow patch, causing OOM when linting or
  # compiling large file sets.  s2-runtime is the same compiler with the
  # heap-grow-patcher applied, so it is functionally equivalent for
  # lint/compile/run invocations.
  for cand in \
    "$REPO_ROOT/.build/selfhost/arukellt-s2-runtime.wasm" \
    "$REPO_ROOT/.build/selfhost/arukellt-s3.wasm" \
    "$REPO_ROOT/.build/selfhost/arukellt-s2.wasm" \
    "$REPO_ROOT/.bootstrap-build/arukellt-s2.wasm" \
    "$REPO_ROOT/.build/selfhost/arukellt-pinned-bootstrap.wasm" \
    "$REPO_ROOT/bootstrap/arukellt-selfhost.wasm"; do
    if [[ -f "$cand" ]]; then echo "$cand"; return 0; fi
  done
  return 1
}

# Retired opt-in: hard-fail with a clear pointer rather than silently
# fall back. Per #583 (ADR-029) the Rust legacy CLI has been removed.
if is_truthy "${ARUKELLT_USE_RUST:-}"; then
  cat >&2 <<'EOF'
arukellt-selfhost: ARUKELLT_USE_RUST is set, but the legacy Rust CLI has
been retired (#583, ADR-029). Selfhost is now the only execution path.
Unset ARUKELLT_USE_RUST and re-run, or invoke the selfhost wasm directly
via `wasmtime run bootstrap/arukellt-selfhost.wasm -- <args>`.
EOF
  exit 2
fi

# #674: manifest-driven component graphs are host-side packaging, like the
# existing `wac plug` path. Intercept this form before resolving the compiler
# wasm so dependency resolution/lock generation works independently of the
# bootstrap compiler binary.
if [[ "${1:-}" == "compose" ]]; then
  for arg in "$@"; do
    if [[ "$arg" == "--manifest" ]]; then
      exec python3 "$REPO_ROOT/scripts/component-deps.py" compose "${@:2}"
    fi
  done
fi

WASMTIME_BIN="${ARUKELLT_WASMTIME_BIN:-wasmtime}"
if ! command -v "$WASMTIME_BIN" >/dev/null 2>&1; then
  echo "arukellt-selfhost: error — wasmtime not found in PATH; install wasmtime ≥ 30" >&2
  exit 127
fi

if ! wasm="$(resolve_selfhost_wasm)"; then
  echo "arukellt-selfhost: error — no selfhost wasm available." >&2
  echo "  Tried: \$ARUKELLT_SELFHOST_WASM, .build/selfhost/arukellt-s3.wasm," >&2
  echo "         .build/selfhost/arukellt-s2.wasm," >&2
  echo "         .bootstrap-build/arukellt-s2.wasm," >&2
  echo "         .build/selfhost/arukellt-pinned-bootstrap.wasm," >&2
  echo "         bootstrap/arukellt-selfhost.wasm" >&2
  echo "  Build one with: python3 scripts/manager.py selfhost fixpoint --build" >&2
  exit 127
fi

if [[ "${1:-}" == "doc" ]]; then
  doc_html=0
  doc_output=""
  i=1
  while [[ $i -le $# ]]; do
    arg="${!i}"
    if [[ "$arg" == "--html" ]]; then
      doc_html=1
      i=$((i + 1))
      continue
    fi
    if [[ "$arg" == "-o" || "$arg" == "--output" ]]; then
      next=$((i + 1))
      if [[ $next -le $# ]]; then
        doc_output="${!next}"
      fi
      i=$((i + 2))
      continue
    fi
    i=$((i + 1))
  done
  if [[ "$doc_html" -eq 1 ]]; then
    exec "$REPO_ROOT/scripts/gen/generate-stdlib-docs.sh" "$doc_output"
  fi
fi

# ADR-050: public `run --target native-cpp` is owned by the host launcher.
# Detect target only from compiler options before `--` so program args cannot
# force or suppress routing (e.g. `run prog.ark -- --target native-cpp`).
is_native_cpp_public_run() {
  local i=2
  local arg next
  while [[ $i -le $# ]]; do
    arg="${!i}"
    if [[ "$arg" == "--" ]]; then
      break
    fi
    if [[ "$arg" == "--target=native-cpp" ]]; then
      return 0
    fi
    if [[ "$arg" == "--target=native-llvm" ]]; then
      return 1
    fi
    if [[ "$arg" == "--target" ]]; then
      next=$((i + 1))
      if [[ $next -le $# ]]; then
        if [[ "${!next}" == "native-cpp" ]]; then
          return 0
        fi
        if [[ "${!next}" == "native-llvm" ]]; then
          return 1
        fi
      fi
    fi
    i=$((i + 1))
  done
  return 1
}

if [[ "${1:-}" == "run" ]] && [[ -z "${ARUKELLT_NATIVE_CPP_INTERNAL_COMPILE:-}" ]] && is_native_cpp_public_run "$@"; then
  exec python3 "$REPO_ROOT/scripts/run/native-cpp-runner.py" "$@"
fi

# #819/#841: direct component compile is a two-stage product operation:
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
  # Driver `run` ignores `--emit component` and writes a core module. For the
  # bridged P2 path (#714), compile to a component then execute with wasmtime.
  emit_mode=""
  i=1
  while [[ $i -le $# ]]; do
    arg="${!i}"
    if [[ "$arg" == "--emit" ]]; then
      next=$((i + 1))
      if [[ $next -le $# ]]; then
        emit_mode="${!next}"
      fi
    fi
    i=$((i + 1))
  done
  if [[ "$emit_mode" == "component" ]]; then
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT
    set +e
    run_compiler "$wasm" compile "${@:2}" \
      >"$tmpdir/stdout" 2>"$tmpdir/stderr"
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
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    plug_runtime_adapter_in_place "$out_path"
    exec wasmtime run --wasm gc --wasm function-references --dir="$REPO_ROOT" "$out_path"
  fi

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT
  set +e
  run_compiler "$wasm" "$@" >"$tmpdir/stdout" 2>"$tmpdir/stderr"
  rc=$?
  set -e
  if [[ "$rc" -ne 0 ]]; then
    cat "$tmpdir/stdout"
    cat "$tmpdir/stderr" >&2
    exit "$rc"
  fi

  out_path="$(sed -n 's/^compiled .* -> //p' "$tmpdir/stderr" | tail -n 1)"
  if [[ -z "$out_path" ]]; then
    cat "$tmpdir/stdout"
    cat "$tmpdir/stderr" >&2
    exit 0
  fi
  if [[ "$out_path" != /* ]]; then
    out_path="$REPO_ROOT/$out_path"
  fi
  # Component Model binaries use version 0x0d; run with wasmtime (bridged P2 path, #714).
  # Do not route them through arukellt-run-hosted.sh just because the payload contains "wasi:".
  wasm_ver="$(od -An -tu1 -j4 -N1 "$out_path" 2>/dev/null | tr -d ' ')"
  if [[ "$wasm_ver" == "13" ]]; then
    plug_runtime_adapter_in_place "$out_path"
    exec wasmtime run --wasm gc --wasm function-references --dir="$REPO_ROOT" "$out_path"
  fi
  exec "$WASMTIME_BIN" run "${WASMTIME_SELFHOST_FLAGS[@]}" --dir="$REPO_ROOT" "$out_path"
fi

# #443 Phase 3: after selfhost validation, delegate binary composition to wac plug.
if [[ "${1:-}" == "compose" ]]; then
  validate_only=0
  for arg in "$@"; do
    if [[ "$arg" == "--validate" ]]; then
      validate_only=1
    fi
  done
  if [[ "$validate_only" -eq 0 ]]; then
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT
    set +e
    "$WASMTIME_BIN" run "${WASMTIME_SELFHOST_FLAGS[@]}" --dir="$REPO_ROOT" "$wasm" -- "$@" >"$tmpdir/stdout" 2>"$tmpdir/stderr"
    rc=$?
    set -e
    cat "$tmpdir/stdout"
    cat "$tmpdir/stderr" >&2
    if [[ "$rc" -ne 0 ]]; then
      exit "$rc"
    fi
    if ! command -v wac >/dev/null 2>&1; then
      echo "error: wac not found in PATH" >&2
      echo "note: binary composition delegates to \`wac plug\` (ADR-034 Phase 3)." >&2
      exit 1
    fi
    plug_provider=""
    plug_socket=""
    plug_output=""
    i=1
    while [[ $i -le $# ]]; do
      arg="${!i}"
      if [[ "$arg" == "--plug" ]]; then
        next=$((i + 1))
        next2=$((i + 2))
        if [[ $next2 -le $# ]]; then
          plug_provider="${!next}"
          plug_socket="${!next2}"
        fi
        i=$((i + 3))
        continue
      fi
      if [[ "$arg" == "-o" || "$arg" == "--output" ]]; then
        next=$((i + 1))
        if [[ $next -le $# ]]; then
          plug_output="${!next}"
        fi
        i=$((i + 2))
        continue
      fi
      i=$((i + 1))
    done
    if [[ -z "$plug_provider" || -z "$plug_socket" || -z "$plug_output" ]]; then
      echo "arukellt-selfhost: error — compose missing --plug provider socket -o output" >&2
      exit 2
    fi
    exec wac plug --plug "$plug_provider" "$plug_socket" -o "$plug_output"
  fi
fi

if [[ "${1:-}" == "debug-adapter" ]]; then
  if [[ "${2:-}" == *.dap-script ]]; then
    if compiler_needs_host_linker "$wasm"; then
      exec "$REPO_ROOT/scripts/run/arukellt-run-hosted.sh" --dir="$REPO_ROOT" "$wasm" -- "$@"
    fi
    exec wasmtime run "${WASMTIME_SELFHOST_FLAGS[@]}" --dir="$REPO_ROOT" "$wasm" -- "$@"
  fi
  DEBUG_ADAPTER="$REPO_ROOT/target/release/arukellt-debug-adapter"
  if [[ ! -x "$DEBUG_ADAPTER" ]]; then
  DEBUG_ADAPTER="$REPO_ROOT/target/debug/arukellt-debug-adapter"
  fi
  if [[ -x "$DEBUG_ADAPTER" ]]; then
    export ARUKELLT_REPO_ROOT="$REPO_ROOT"
    exec "$DEBUG_ADAPTER" "${@:2}"
  fi
fi

# Default: fmt / lint / compile / version / … (#834 host-linker for wasi-p2).
if compiler_needs_host_linker "$wasm"; then
  exec "$REPO_ROOT/scripts/run/arukellt-run-hosted.sh" --dir="$REPO_ROOT" "$wasm" -- "$@"
fi
exec wasmtime run "${WASMTIME_SELFHOST_FLAGS[@]}" --dir="$REPO_ROOT" "$wasm" -- "$@"
