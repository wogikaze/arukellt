#!/usr/bin/env bash
# scripts/run/arukellt-selfhost.sh — Selfhost-only arukellt entrypoint (#559, #583).
#
# This wrapper executes the **selfhost wasm** for every invocation. Per #583
# (and ADR-029, #585), the compiler and runtime entrypoint are self-hosted.
#
# Resolution order (each existing candidate is validated before selection):
#   1. `$ARUKELLT_SELFHOST_WASM`
#   2. `bootstrap/arukellt-selfhost.wasm` (committed pinned reference;
#      see `bootstrap/PROVENANCE.md`)
#   3. `.build/selfhost/arukellt-s2-runtime.wasm`
#   4. `.build/selfhost/arukellt-s3.wasm` (stage-3 self-compile)
#   5. `.build/selfhost/arukellt-s2.wasm`
#   6. `.bootstrap-build/arukellt-s2.wasm`
# A WASI P1 core runs directly. A standard WASI P2 core is packaged with the
# official wasm-tools WIT package before it runs. Retired bridge/legacy ABI
# artifacts are rejected rather than adapted.
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

SELFHOST_RUN_KIND="core"

run_compiler() {
  local wasm_path="$1"
  shift
  if [[ "$SELFHOST_RUN_KIND" == "component" ]]; then
    "$WASMTIME_BIN" run --wasm gc --wasm function-references -W memory64=y \
      --dir="$REPO_ROOT" "$wasm_path" -- "$@"
    return
  fi
  "$WASMTIME_BIN" run "${WASMTIME_SELFHOST_FLAGS[@]}" --dir="$REPO_ROOT" "$wasm_path" -- "$@"
}

resolve_selfhost_wasm() {
  if [[ -n "${ARUKELLT_SELFHOST_WASM:-}" ]]; then
    printf '%s\n' "$ARUKELLT_SELFHOST_WASM"
    return 0
  fi
  printf '%s\n' \
    "$REPO_ROOT/bootstrap/arukellt-selfhost.wasm" \
    "$REPO_ROOT/.build/selfhost/arukellt-s2-runtime.wasm" \
    "$REPO_ROOT/.build/selfhost/arukellt-s3.wasm" \
    "$REPO_ROOT/.build/selfhost/arukellt-s2.wasm" \
    "$REPO_ROOT/.bootstrap-build/arukellt-s2.wasm" \
    "$REPO_ROOT/.build/selfhost/arukellt-pinned-bootstrap.wasm"
}

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

WASM_TOOLS_BIN="${ARUKELLT_WASM_TOOLS_BIN:-wasm-tools}"
WASI_P2_WIT_DIR="${ARUKELLT_WASI_P2_WIT_DIR:-$REPO_ROOT/scripts/selfhost/wit/deps/wasi-cli-0.2.0/wit}"

wasm_artifact_version() {
  od -An -tu1 -j4 -N1 "$1" 2>/dev/null | tr -d ' '
}

has_retired_abi() {
  local artifact="$1"
  local printed
  if ! command -v "$WASM_TOOLS_BIN" >/dev/null 2>&1; then
    return 1
  fi
  printed="$($WASM_TOOLS_BIN print "$artifact" 2>/dev/null)" || return 1
  if grep -Eq '\(import "(arukellt:|runtime/host)' <<<"$printed"; then
    return 0
  fi
  if grep -Eq \
    '\(import "wasi:(cli/(stdout|stderr|stdin|environment)|filesystem/types)@0\.2\.0" "(write|read|args-sizes|arguments|environ-sizes|environ-get|open-at|close)"' \
    <<<"$printed"; then
    return 0
  fi
  return 1
}

has_core_import_module() {
  local artifact="$1"
  local module="$2"
  local printed
  if ! command -v "$WASM_TOOLS_BIN" >/dev/null 2>&1; then
    return 1
  fi
  printed="$($WASM_TOOLS_BIN print "$artifact" 2>/dev/null)" || return 1
  grep -Fq "(import \"${module}\"" <<<"$printed"
}

has_library_exports() {
  local artifact="$1"
  local printed
  if ! command -v "$WASM_TOOLS_BIN" >/dev/null 2>&1; then
    return 1
  fi
  printed="$($WASM_TOOLS_BIN print "$artifact" 2>/dev/null)" || return 1
  if grep -Eq '\(export "cm32p2\|\|(run|run_post)"' <<<"$printed"; then
    return 1
  fi
  grep -Eq '\(export "cm32p2\|\|[^\"]+"' <<<"$printed"
}

prepare_selfhost_artifact() {
  local input="$1"
  local version
  version="$(wasm_artifact_version "$input")"
  case "$version" in
    13)
      require_wasm_tools || return 1
      if has_retired_abi "$input"; then
        echo "arukellt-selfhost: error — selected component uses a retired bridge/legacy ABI" >&2
        return 1
      fi
      SELFHOST_RUN_KIND="component"
      SELFHOST_RUN_PATH="$input"
      return 0
      ;;
    1)
      if has_core_import_module "$input" "wasi_snapshot_preview1"; then
        SELFHOST_RUN_KIND="core"
        SELFHOST_RUN_PATH="$input"
        return 0
      fi
      if has_retired_abi "$input"; then
        echo "arukellt-selfhost: error — selected core uses a retired bridge/legacy ABI" >&2
        return 1
      fi
      if ! has_core_import_module "$input" "cm32p2|"; then
        echo "arukellt-selfhost: error — selected core has no official WASI P1 or P2 ABI" >&2
        return 1
      fi
      require_wasm_tools || return 1
      local package_dir="$REPO_ROOT/.build/selfhost/official-p2"
      local stem
      stem="$(basename "$input" .wasm)"
      mkdir -p "$package_dir"
      local output="$package_dir/$stem.component.wasm"
      local embedded="$package_dir/$stem.embedded.wasm"
      if [[ ! -f "$output" || "$output" -ot "$input" || "$output" -ot "$WASI_P2_WIT_DIR" ]]; then
        "$WASM_TOOLS_BIN" component embed "$WASI_P2_WIT_DIR" --world command \
          "$input" -o "$embedded"
        "$WASM_TOOLS_BIN" component new "$embedded" --reject-legacy-names \
          --realloc-via-memory-grow -o "$output"
      fi
      if has_retired_abi "$output"; then
        echo "arukellt-selfhost: error — official P2 packaging retained a retired ABI" >&2
        return 1
      fi
      SELFHOST_RUN_KIND="component"
      SELFHOST_RUN_PATH="$output"
      return 0
      ;;
    *)
      echo "arukellt-selfhost: error — selected selfhost artifact is not a Wasm core/component" >&2
      return 1
      ;;
  esac
}

select_selfhost_artifact() {
  local candidate
  while IFS= read -r candidate; do
    if [[ ! -f "$candidate" ]]; then
      continue
    fi
    if prepare_selfhost_artifact "$candidate"; then
      return 0
    fi
  done < <(resolve_selfhost_wasm)
  echo "arukellt-selfhost: error — no usable selfhost wasm available." >&2
  echo "  Tried: \$ARUKELLT_SELFHOST_WASM, bootstrap/arukellt-selfhost.wasm," >&2
  echo "         .build/selfhost/arukellt-s2-runtime.wasm, .build/selfhost/arukellt-s3.wasm," >&2
  echo "         .build/selfhost/arukellt-s2.wasm, .bootstrap-build/arukellt-s2.wasm," >&2
  echo "         .build/selfhost/arukellt-pinned-bootstrap.wasm" >&2
  echo "  Build one with: python3 scripts/manager.py selfhost fixpoint --build" >&2
  return 1
}

component_emit_requested() {
  local i=1
  local arg next
  while [[ $i -le $# ]]; do
    arg="${!i}"
    if [[ "$arg" == "--emit=component" ]]; then
      return 0
    fi
    if [[ "$arg" == "--emit" ]]; then
      next=$((i + 1))
      if [[ $next -le $# && "${!next}" == "component" ]]; then
        return 0
      fi
      i=$((i + 2))
      continue
    fi
    i=$((i + 1))
  done
  return 1
}

emit_all_requested() {
  local i=1
  local arg next
  while [[ $i -le $# ]]; do
    arg="${!i}"
    if [[ "$arg" == "--emit=all" ]]; then
      return 0
    fi
    if [[ "$arg" == "--emit" ]]; then
      next=$((i + 1))
      if [[ $next -le $# && "${!next}" == "all" ]]; then
        return 0
      fi
      i=$((i + 2))
      continue
    fi
    i=$((i + 1))
  done
  return 1
}

component_output_path_from_args() {
  P2_COMPONENT_OUTPUT=""
  P2_COMPONENT_OUTPUT_EXPLICIT=0
  P2_COMPONENT_INPUT=""
  local i=1
  local arg next
  while [[ $i -le $# ]]; do
    arg="${!i}"
    if [[ "$arg" == "-o" || "$arg" == "--output" ]]; then
      next=$((i + 1))
      if [[ $next -le $# && "${!next}" != "json" ]]; then
        P2_COMPONENT_OUTPUT="${!next}"
        P2_COMPONENT_OUTPUT_EXPLICIT=1
      fi
      i=$((i + 2))
      continue
    fi
    if [[ "$arg" == *.ark && -z "$P2_COMPONENT_INPUT" ]]; then
      P2_COMPONENT_INPUT="$arg"
    fi
    i=$((i + 1))
  done
  if [[ "$P2_COMPONENT_OUTPUT_EXPLICIT" -eq 0 && -n "$P2_COMPONENT_INPUT" ]]; then
    if [[ "$P2_COMPONENT_INPUT" == *.ark ]]; then
      P2_COMPONENT_OUTPUT="${P2_COMPONENT_INPUT%.ark}.component.wasm"
    else
      P2_COMPONENT_OUTPUT="$P2_COMPONENT_INPUT.component.wasm"
    fi
  fi
}

prepare_component_core_args() {
  local core_path="$1"
  shift
  P2_CORE_ARGS=()
  local i=1
  local arg next
  while [[ $i -le $# ]]; do
    arg="${!i}"
    if [[ "$arg" == "-o" || "$arg" == "--output" ]]; then
      next=$((i + 1))
      if [[ $next -le $# && "${!next}" == "json" ]]; then
        P2_CORE_ARGS+=("$arg" "${!next}")
      fi
      i=$((i + 2))
      continue
    fi
    if [[ "$arg" == "--emit=component" ]]; then
      P2_CORE_ARGS+=(--emit component)
      i=$((i + 1))
      continue
    fi
    P2_CORE_ARGS+=("$arg")
    i=$((i + 1))
  done
  P2_CORE_ARGS+=("-o" "$core_path")
}

prepare_component_wit_args() {
  local wit_path="$1"
  local wit_world="$2"
  shift 2
  P2_WIT_ARGS=()
  local i=1
  local arg next
  while [[ $i -le $# ]]; do
    arg="${!i}"
    if [[ "$arg" == "-o" || "$arg" == "--output" ]]; then
      next=$((i + 1))
      if [[ $next -le $# && "${!next}" == "json" ]]; then
        P2_WIT_ARGS+=("$arg" "${!next}")
      fi
      i=$((i + 2))
      continue
    fi
    if [[ "$arg" == "--emit=component" || "$arg" == "--emit=all" ]]; then
      P2_WIT_ARGS+=(--emit wit)
      i=$((i + 1))
      continue
    fi
    if [[ "$arg" == "--emit" ]]; then
      next=$((i + 1))
      if [[ $next -le $# && ("${!next}" == "component" || "${!next}" == "all") ]]; then
        P2_WIT_ARGS+=(--emit wit)
      else
        P2_WIT_ARGS+=("$arg" "${!next}")
      fi
      i=$((i + 2))
      continue
    fi
    P2_WIT_ARGS+=("$arg")
    i=$((i + 1))
  done
  if [[ "$wit_world" == "wasi:cli/command" ]]; then
    P2_WIT_ARGS+=(--world "$wit_world")
  fi
  P2_WIT_ARGS+=("-o" "$(guest_path_for_selfhost "$wit_path")")
}

all_output_paths_from_args() {
  ALL_WASM_OUTPUT=""
  ALL_COMPONENT_OUTPUT=""
  ALL_WASM_OUTPUT_EXPLICIT=0
  ALL_INPUT=""
  local i=1
  local arg next
  while [[ $i -le $# ]]; do
    arg="${!i}"
    if [[ "$arg" == "-o" || "$arg" == "--output" ]]; then
      next=$((i + 1))
      if [[ $next -le $# && "${!next}" != "json" ]]; then
        ALL_WASM_OUTPUT="${!next}"
        ALL_WASM_OUTPUT_EXPLICIT=1
      fi
      i=$((i + 2))
      continue
    fi
    if [[ "$arg" == *.ark && -z "$ALL_INPUT" ]]; then
      ALL_INPUT="$arg"
    fi
    i=$((i + 1))
  done
  if [[ "$ALL_WASM_OUTPUT_EXPLICIT" -eq 0 && -n "$ALL_INPUT" ]]; then
    ALL_WASM_OUTPUT="${ALL_INPUT%.ark}.wasm"
  fi
  if [[ -z "$ALL_WASM_OUTPUT" ]]; then
    return 0
  fi
  if [[ "$ALL_WASM_OUTPUT" == *.wasm ]]; then
    ALL_COMPONENT_OUTPUT="${ALL_WASM_OUTPUT%.wasm}.component.wasm"
  else
    ALL_COMPONENT_OUTPUT="$ALL_WASM_OUTPUT.component.wasm"
  fi
}

prepare_all_args() {
  local core_path="$1"
  shift
  ALL_CORE_ARGS=()
  local i=1
  local arg next
  while [[ $i -le $# ]]; do
    arg="${!i}"
    if [[ "$arg" == "-o" || "$arg" == "--output" ]]; then
      next=$((i + 1))
      if [[ $next -le $# && "${!next}" == "json" ]]; then
        ALL_CORE_ARGS+=("$arg" "${!next}")
      fi
      i=$((i + 2))
      continue
    fi
    if [[ "$arg" == "--emit=all" ]]; then
      ALL_CORE_ARGS+=(--emit all)
      i=$((i + 1))
      continue
    fi
    ALL_CORE_ARGS+=("$arg")
    i=$((i + 1))
  done
  ALL_CORE_ARGS+=("-o" "$core_path")
}

require_wasm_tools() {
  if ! command -v "$WASM_TOOLS_BIN" >/dev/null 2>&1; then
    echo "arukellt-selfhost: error — wasm-tools is required for official WASI P2 component packaging" >&2
    return 1
  fi
  if [[ ! -d "$WASI_P2_WIT_DIR" ]]; then
    echo "arukellt-selfhost: error — official WASI P2 WIT directory not found: $WASI_P2_WIT_DIR" >&2
    return 1
  fi
}

prepare_component_wit_tree() {
  local wit_source="$1"
  local tree="$2"
  shift 2
  mkdir -p "$tree/deps/wasi-cli"
  /bin/cp -f "$wit_source"/*.wit "$tree/deps/wasi-cli/"
  if [[ -f "$wit_source/deps.toml" ]]; then
    /bin/cp -f "$wit_source/deps.toml" "$tree/deps/wasi-cli/deps.toml"
  fi
  local dep
  for dep in clocks filesystem io random sockets; do
    mkdir -p "$tree/deps/$dep"
    /bin/cp -f "$wit_source/deps/$dep"/*.wit "$tree/deps/$dep/"
  done
  local wit_index=0
  local copied_sources=$'\n'
  local i=1
  local arg next source source_host dependency
  local input_file=""
  local input_dir=""
  local vendor_source
  while [[ $i -le $# ]]; do
    arg="${!i}"
    if [[ "$arg" == *.ark && -z "$input_file" ]]; then
      input_file="$arg"
    fi
    source=""
    if [[ "$arg" == "--wit" ]]; then
      next=$((i + 1))
      if [[ $next -le $# ]]; then
        source="${!next}"
      fi
      i=$((i + 2))
    elif [[ "$arg" == --wit=* ]]; then
      source="${arg#--wit=}"
      i=$((i + 1))
    else
      i=$((i + 1))
      continue
    fi
    if [[ -z "$source" ]]; then
      continue
    fi
    if [[ "$source" == /* ]]; then
      source_host="$source"
    else
      source_host="$REPO_ROOT/$source"
    fi
    case "$copied_sources" in
      *$'\n'"$source_host"$'\n'*)
        continue
        ;;
    esac
    dependency="$tree/deps/user-$wit_index"
    mkdir -p "$dependency"
    if [[ -d "$source_host" ]]; then
      local -a wit_files=("$source_host"/*.wit)
      if [[ ${#wit_files[@]} -eq 0 || ! -f "${wit_files[0]}" ]]; then
        continue
      fi
      /bin/cp -f "${wit_files[@]}" "$dependency/"
    else
      /bin/cp -f "$source_host" "$dependency/"
    fi
    copied_sources+="$source_host"$'\n'
    wit_index=$((wit_index + 1))
  done
  if [[ -n "$input_file" ]]; then
    if [[ "$input_file" == /* ]]; then
      input_dir="$(dirname "$input_file")"
    else
      input_dir="$REPO_ROOT/$(dirname "$input_file")"
    fi
    if [[ -d "$input_dir/vendor" ]]; then
      for vendor_source in "$input_dir"/vendor/*; do
        if [[ ! -d "$vendor_source" ]]; then
          continue
        fi
        case "$copied_sources" in
          *$'\n'"$vendor_source"$'\n'*)
            continue
            ;;
        esac
        local -a vendor_wit_files=("$vendor_source"/*.wit)
        if [[ ${#vendor_wit_files[@]} -eq 0 || ! -f "${vendor_wit_files[0]}" ]]; then
          continue
        fi
        dependency="$tree/deps/user-$wit_index"
        mkdir -p "$dependency"
        /bin/cp -f "${vendor_wit_files[@]}" "$dependency/"
        copied_sources+="$vendor_source"$'\n'
        wit_index=$((wit_index + 1))
      done
    fi
  fi
}

make_selfhost_tempdir() {
  mkdir -p "$REPO_ROOT/.build"
  mktemp -d "$REPO_ROOT/.build/arukellt-selfhost.XXXXXX"
}

guest_path_for_selfhost() {
  local host_path="$1"
  if [[ "$host_path" != "$REPO_ROOT/"* ]]; then
    echo "arukellt-selfhost: error — temporary compiler output is outside the repo preopen" >&2
    return 1
  fi
  printf '%s\n' "${host_path#"$REPO_ROOT/"}"
}

package_component_core() {
  local core_path="$1"
  local output_path="$2"
  local work_dir="$3"
  local component_wit="${4:-}"
  local wit_world="${5:-}"
  shift 5
  local wasm_version
  wasm_version="$(od -An -tu1 -j4 -N1 "$core_path" 2>/dev/null | tr -d ' ')"
  if [[ "$wasm_version" == "13" ]]; then
    /bin/cp -f "$core_path" "$output_path"
    P2_COMPONENT_RESULT="$output_path"
    return 0
  fi
  if [[ "$wasm_version" != "1" ]]; then
    echo "arukellt-selfhost: error — component emission produced an unknown Wasm artifact" >&2
    return 1
  fi
  require_wasm_tools || return 1
  local embedded="$work_dir/component-embedded.wasm"
  if [[ -z "$component_wit" || ! -s "$component_wit" ]]; then
    echo "arukellt-selfhost: error — component core has no generated WIT world" >&2
    return 1
  fi
  local component_wit_tree="$work_dir/component-wit"
  mkdir -p "$component_wit_tree"
  /bin/cp -f "$component_wit" "$component_wit_tree/arukellt.wit"
  prepare_component_wit_tree "$WASI_P2_WIT_DIR" "$component_wit_tree" "$@"
  if [[ -z "$wit_world" ]]; then
    echo "arukellt-selfhost: error — component WIT world is unspecified" >&2
    return 1
  fi
  "$WASM_TOOLS_BIN" component embed "$component_wit_tree" --world "$wit_world" \
    "$core_path" -o "$embedded"
  "$WASM_TOOLS_BIN" component new "$embedded" --reject-legacy-names \
    --realloc-via-memory-grow -o "$output_path"
  P2_COMPONENT_RESULT="$output_path"
}

compile_component_artifact() {
  local work_dir="$1"
  shift
  component_output_path_from_args "$@"
  if [[ -z "$P2_COMPONENT_OUTPUT" ]]; then
    echo "arukellt-selfhost: error — component compile needs an input .ark file or -o output path" >&2
    return 2
  fi
  local core_path="$work_dir/core.wasm"
  local guest_core_path
  guest_core_path="$(guest_path_for_selfhost "$core_path")"
  prepare_component_core_args "$guest_core_path" "$@"
  if ! run_compiler "$wasm" "${P2_CORE_ARGS[@]}"; then
    return 1
  fi
  if [[ ! -f "$core_path" ]]; then
    echo "arukellt-selfhost: error — compiler did not produce core Wasm at $core_path" >&2
    return 1
  fi
  if has_retired_abi "$core_path"; then
    echo "arukellt-selfhost: error — compiler produced a retired bridge/legacy ABI" >&2
    return 1
  fi
  if ! "$WASM_TOOLS_BIN" validate "$core_path" >/dev/null 2>&1; then
    echo "arukellt-selfhost: error — compiler produced invalid core Wasm" >&2
    return 1
  fi
  local component_wit="$work_dir/component.wit"
  local compiler_world="wasi:cli/command"
  local wit_world="command"
  if has_library_exports "$core_path"; then
    compiler_world=""
    wit_world="arukellt"
  fi
  prepare_component_wit_args "$component_wit" "$compiler_world" "$@"
  if ! run_compiler "$wasm" "${P2_WIT_ARGS[@]}"; then
    echo "arukellt-selfhost: error — compiler could not produce the component WIT world" >&2
    return 1
  fi
  if [[ ! -s "$component_wit" ]]; then
    echo "arukellt-selfhost: error — compiler did not produce component WIT at $component_wit" >&2
    return 1
  fi
  package_component_core "$core_path" "$P2_COMPONENT_OUTPUT" "$work_dir" "$component_wit" "$wit_world" "$@"
  if ! "$WASM_TOOLS_BIN" validate "$P2_COMPONENT_OUTPUT" >/dev/null 2>&1; then
    echo "arukellt-selfhost: error — official component packaging produced invalid Wasm" >&2
    return 1
  fi
  echo "component written to $P2_COMPONENT_OUTPUT" >&2
}

compile_all_artifact() {
  local work_dir="$1"
  shift
  all_output_paths_from_args "$@"
  if [[ -z "$ALL_WASM_OUTPUT" || -z "$ALL_COMPONENT_OUTPUT" ]]; then
    echo "arukellt-selfhost: error — --emit all needs an input .ark file or -o output path" >&2
    return 2
  fi
  local core_path="$work_dir/all.wasm"
  local component_core_path="$work_dir/all.component.wasm"
  local guest_core_path
  guest_core_path="$(guest_path_for_selfhost "$core_path")"
  prepare_all_args "$guest_core_path" "$@"
  if ! run_compiler "$wasm" "${ALL_CORE_ARGS[@]}"; then
    return 1
  fi
  if [[ ! -f "$core_path" ]]; then
    echo "arukellt-selfhost: error — compiler did not produce core Wasm at $core_path" >&2
    return 1
  fi
  if has_retired_abi "$core_path"; then
    echo "arukellt-selfhost: error — compiler produced a retired bridge/legacy ABI" >&2
    return 1
  fi
  if ! "$WASM_TOOLS_BIN" validate "$core_path" >/dev/null 2>&1; then
    echo "arukellt-selfhost: error — compiler produced invalid core Wasm" >&2
    return 1
  fi
  if [[ ! -f "$component_core_path" ]]; then
    echo "arukellt-selfhost: error — compiler did not produce component Wasm at $component_core_path" >&2
    return 1
  fi
  /bin/cp -f "$core_path" "$ALL_WASM_OUTPUT"
  local component_wit="$work_dir/component.wit"
  local compiler_world="wasi:cli/command"
  local wit_world="command"
  if has_library_exports "$component_core_path"; then
    compiler_world=""
    wit_world="arukellt"
  fi
  prepare_component_wit_args "$component_wit" "$compiler_world" "$@"
  if ! run_compiler "$wasm" "${P2_WIT_ARGS[@]}"; then
    echo "arukellt-selfhost: error — compiler could not produce the component WIT world" >&2
    return 1
  fi
  if [[ ! -s "$component_wit" ]]; then
    echo "arukellt-selfhost: error — compiler did not produce component WIT at $component_wit" >&2
    return 1
  fi
  package_component_core "$component_core_path" "$ALL_COMPONENT_OUTPUT" "$work_dir" "$component_wit" "$wit_world" "$@"
  if ! "$WASM_TOOLS_BIN" validate "$ALL_COMPONENT_OUTPUT" >/dev/null 2>&1; then
    echo "arukellt-selfhost: error — official component packaging produced invalid Wasm" >&2
    return 1
  fi
  echo "Wasm written to $ALL_WASM_OUTPUT" >&2
  echo "component written to $ALL_COMPONENT_OUTPUT" >&2
}

if ! select_selfhost_artifact; then
  exit 1
fi
wasm="$SELFHOST_RUN_PATH"

# P2 command components are produced by the standard wasm-tools component
# pipeline. The selfhost compiler emits either the already-built library
# component or a core module with the standard cm32p2 ABI metadata.
if [[ "${1:-}" == "compile" ]] && component_emit_requested "$@"; then
  tmpdir="$(make_selfhost_tempdir)"
  trap 'rm -rf "$tmpdir"' EXIT
  compile_component_artifact "$tmpdir" "$@"
  exit $?
fi

if [[ "${1:-}" == "compile" ]] && emit_all_requested "$@"; then
  tmpdir="$(make_selfhost_tempdir)"
  trap 'rm -rf "$tmpdir"' EXIT
  compile_all_artifact "$tmpdir" "$@"
  exit $?
fi

# `component build` is an alias implemented by the selfhost CLI. Route its
# command-only P2 output through the same official packaging path.
if [[ "${1:-}" == "component" && "${2:-}" != "inspect" && "${2:-}" != "validate" ]]; then
  tmpdir="$(make_selfhost_tempdir)"
  trap 'rm -rf "$tmpdir"' EXIT
  component_build_args=(compile)
  if [[ "${2:-}" == "build" ]]; then
    component_build_args+=("${@:3}")
  else
    component_build_args+=("${@:2}")
  fi
  component_build_args+=(--target wasm32-gc --wasi-version wasi-p2 --emit component)
  compile_component_artifact "$tmpdir" "${component_build_args[@]}"
  exit $?
fi

if [[ "${1:-}" == "run" ]]; then
  emit_mode=""
  i=1
  while [[ $i -le $# ]]; do
    arg="${!i}"
    if [[ "$arg" == "--emit=component" ]]; then
      emit_mode="component"
    fi
    if [[ "$arg" == "--emit" ]]; then
      next=$((i + 1))
      if [[ $next -le $# ]]; then
        emit_mode="${!next}"
      fi
    fi
    i=$((i + 1))
  done
  if [[ "$emit_mode" == "component" ]]; then
    tmpdir="$(make_selfhost_tempdir)"
    trap 'rm -rf "$tmpdir"' EXIT
    run_component_args=(compile "${@:2}")
    compile_component_artifact "$tmpdir" "${run_component_args[@]}"
    out_path="$P2_COMPONENT_RESULT"
    exec "$WASMTIME_BIN" run --wasm gc --wasm function-references --dir="$REPO_ROOT" "$out_path"
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
  # Component Model binaries use version 0x0d and can be run directly.
  wasm_ver="$(od -An -tu1 -j4 -N1 "$out_path" 2>/dev/null | tr -d ' ')"
  if [[ "$wasm_ver" == "13" ]]; then
    exec "$WASMTIME_BIN" run --wasm gc --wasm function-references --dir="$REPO_ROOT" "$out_path"
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
    exec "$WASMTIME_BIN" run "${WASMTIME_SELFHOST_FLAGS[@]}" --dir="$REPO_ROOT" "$wasm" -- "$@"
  fi
  echo "arukellt-selfhost: debug-adapter requires a .dap-script" >&2
  exit 2
fi

# Default: fmt / lint / compile / version / …, always through direct Wasmtime.
exec "$WASMTIME_BIN" run "${WASMTIME_SELFHOST_FLAGS[@]}" --dir="$REPO_ROOT" "$wasm" -- "$@"
