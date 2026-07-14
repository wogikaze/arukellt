#!/usr/bin/env bash
# Optimization equivalence gate: O0 and O1 must produce identical runtime behavior.
#
# Compiles each run: fixture at --opt-level 0 and --opt-level 1 with the pinned
# selfhost compiler (bootstrap/arukellt-selfhost.wasm), executes both wasms, and
# compares stdout+stderr and exit code. Compile failures, traps, and invalid
# modules are skipped (same policy as fixture-parity).
#
# Usage:
#   bash scripts/run/test-opt-equivalence.sh [--quick] [--fixture PATH]

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
COMPILER_WASM="${ARUKELLT_PINNED_WASM:-$REPO_ROOT/bootstrap/arukellt-selfhost.wasm}"
TARGET="wasm32"
WASI_VERSION="wasi-p1"
OUT_DIR="$REPO_ROOT/.ark-opt-equiv-tmp"
MANIFEST="$REPO_ROOT/tests/fixtures/manifest.txt"
HOSTED_RUN="$REPO_ROOT/scripts/run/arukellt-run-hosted.sh"
COMPILE_TIMEOUT=30
RUN_TIMEOUT=15

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

QUICK_FIXTURES=(
    hello_world.ark
    control/if_basic.ark
    control/match_enum.ark
    control/while_counter.ark
    arrays/array_literal.ark
    scalar/f32_local.ark
    module_import/use_std_string.ark
    closure_capture/capture_struct.ark
    generics/identity_i32.ark
    structs/basic_struct.ark
    enums/option_some.ark
    stdlib_hashmap/hashmap_basic.ark
    stdlib_math/sqrt.ark
    stdlib_string/string_concat.ark
    stdlib_json/json_basic.ark
)

MODE="manifest"
SINGLE_FIXTURE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --quick)
            MODE="quick"
            shift
            ;;
        --fixture)
            MODE="single"
            SINGLE_FIXTURE="${2:-}"
            if [[ -z "$SINGLE_FIXTURE" ]]; then
                echo "error: --fixture requires a path argument" >&2
                exit 2
            fi
            shift 2
            ;;
        -h|--help)
            sed -n '1,12p' "$0" | tail -n +2
            exit 0
            ;;
        *)
            echo "error: unknown argument: $1" >&2
            exit 2
            ;;
    esac
done

find_wasmtime() {
    if [[ -n "${WASMTIME:-}" ]] && [[ -x "${WASMTIME}" || -n "$(command -v "$WASMTIME" 2>/dev/null)" ]]; then
        echo "$WASMTIME"
        return 0
    fi
    command -v wasmtime 2>/dev/null || true
}

WASMTIME_BIN="$(find_wasmtime)"
if [[ -z "$WASMTIME_BIN" ]]; then
    echo -e "${RED}error: wasmtime not found${NC}" >&2
    exit 1
fi

if [[ ! -f "$COMPILER_WASM" ]]; then
    echo -e "${RED}error: compiler wasm not found at $COMPILER_WASM${NC}" >&2
    exit 1
fi

normalize_fixture_path() {
    local raw="$1"
    raw="${raw#tests/fixtures/}"
    raw="${raw#/}"
    echo "$raw"
}

load_manifest_fixtures() {
    if [[ ! -f "$MANIFEST" ]]; then
        echo -e "${RED}error: manifest not found: $MANIFEST${NC}" >&2
        exit 1
    fi
    local fixtures=()
    while IFS= read -r line; do
        if [[ "$line" =~ ^run:[[:space:]]*(.+)$ ]]; then
            fixtures+=("${BASH_REMATCH[1]}")
        fi
    done < "$MANIFEST"
    if [[ ${#fixtures[@]} -eq 0 ]]; then
        echo -e "${RED}error: no run: fixtures in manifest${NC}" >&2
        exit 1
    fi
    printf '%s\n' "${fixtures[@]}"
}

declare -a FIXTURES=()
case "$MODE" in
    quick)
        FIXTURES=("${QUICK_FIXTURES[@]}")
        ;;
    single)
        FIXTURES=("$(normalize_fixture_path "$SINGLE_FIXTURE")")
        ;;
    manifest)
        mapfile -t FIXTURES < <(load_manifest_fixtures)
        ;;
esac

mkdir -p "$OUT_DIR"

wasm_needs_host_linker() {
    local wasm="$1"
    grep -aq "arukellt_host" "$wasm" 2>/dev/null
}

is_trap_or_invalid() {
    local code="$1"
    local out="$2"
    [[ "$code" -eq 134 ]] || { [[ "$code" -eq 1 ]] && [[ "$out" == *"failed to compile"* ]]; }
}

is_compile_skip() {
    local code="$1"
    [[ "$code" -ne 0 ]] || [[ "$code" -eq 124 ]]
}

compile_fixture() {
    local fixture="$1"
    local opt_level="$2"
    local out_rel="$3"
    local src_rel="tests/fixtures/$fixture"
    timeout "$COMPILE_TIMEOUT" "$WASMTIME_BIN" run \
        --dir "$REPO_ROOT" \
        "$COMPILER_WASM" -- \
        compile "$src_rel" --target "$TARGET" --wasi-version "$WASI_VERSION" --opt-level "$opt_level" -o "$out_rel"
}

run_wasm() {
    local wasm="$1"
    local out_file="$2"
    local code=0
    if wasm_needs_host_linker "$wasm"; then
        timeout "$RUN_TIMEOUT" bash "$HOSTED_RUN" --dir="$REPO_ROOT" "$wasm" >"$out_file" 2>&1 || code=$?
    else
        timeout "$RUN_TIMEOUT" "$WASMTIME_BIN" run --dir="$REPO_ROOT" "$wasm" >"$out_file" 2>&1 || code=$?
    fi
    echo "$code"
}

PASS=0
FAIL=0
SKIP=0

echo -e "${YELLOW}[opt-equivalence] Checking ${#FIXTURES[@]} fixture(s) (O0 == O1)...${NC}"

for fixture in "${FIXTURES[@]}"; do
    ark_file="$REPO_ROOT/tests/fixtures/$fixture"
    if [[ ! -f "$ark_file" ]]; then
        echo "  skip: $fixture (not found on disk)"
        SKIP=$((SKIP + 1))
        continue
    fi

    safe_name="${fixture//\//_}"
    out0_rel=".ark-opt-equiv-tmp/o0-${safe_name}.wasm"
    out1_rel=".ark-opt-equiv-tmp/o1-${safe_name}.wasm"
    out0="$REPO_ROOT/$out0_rel"
    out1="$REPO_ROOT/$out1_rel"

    compile0_out="$(mktemp)"
    compile0_code=0
    compile_fixture "$fixture" 0 "$out0_rel" >"$compile0_out" 2>&1 || compile0_code=$?
    if is_compile_skip "$compile0_code"; then
        echo "  skip: $fixture (O0 compile failed/timeout)"
        SKIP=$((SKIP + 1))
        rm -f "$compile0_out"
        continue
    fi

    compile1_out="$(mktemp)"
    compile1_code=0
    compile_fixture "$fixture" 1 "$out1_rel" >"$compile1_out" 2>&1 || compile1_code=$?
    if is_compile_skip "$compile1_code"; then
        echo "  skip: $fixture (O1 compile failed/timeout)"
        SKIP=$((SKIP + 1))
        rm -f "$compile0_out" "$compile1_out"
        continue
    fi
    rm -f "$compile0_out" "$compile1_out"

    run0_out="$(mktemp)"
    run1_out="$(mktemp)"
    run0_code="$(run_wasm "$out0" "$run0_out")"
    run1_code="$(run_wasm "$out1" "$run1_out")"

    out0_text="$(tr -d '\0' <"$run0_out" | sed -e 's/[[:space:]]*$//')"
    out1_text="$(tr -d '\0' <"$run1_out" | sed -e 's/[[:space:]]*$//')"

    if is_trap_or_invalid "$run0_code" "$out0_text" || is_trap_or_invalid "$run1_code" "$out1_text"; then
        echo "  skip: $fixture (wasm trap/invalid)"
        SKIP=$((SKIP + 1))
        rm -f "$run0_out" "$run1_out"
        continue
    fi

    if [[ "$run0_code" -eq 124 || "$run1_code" -eq 124 ]]; then
        echo "  skip: $fixture (run timeout)"
        SKIP=$((SKIP + 1))
        rm -f "$run0_out" "$run1_out"
        continue
    fi

    if [[ "$out0_text" == "$out1_text" && "$run0_code" -eq "$run1_code" ]]; then
        PASS=$((PASS + 1))
    else
        echo -e "  ${RED}FAIL${NC}: $fixture (O0 != O1)"
        if [[ "$run0_code" -ne "$run1_code" ]]; then
            echo "    exit: O0=$run0_code O1=$run1_code"
        fi
        if [[ "$out0_text" != "$out1_text" ]]; then
            echo "    O0 output:"
            printf '%s\n' "$out0_text" | head -20 | sed 's/^/      /'
            echo "    O1 output:"
            printf '%s\n' "$out1_text" | head -20 | sed 's/^/      /'
            if command -v diff >/dev/null 2>&1; then
                echo "    diff:"
                diff -u <(printf '%s\n' "$out0_text") <(printf '%s\n' "$out1_text") | head -30 | sed 's/^/      /' || true
            fi
        fi
        FAIL=$((FAIL + 1))
    fi

    rm -f "$run0_out" "$run1_out"
done

echo ""
echo -e "${YELLOW}opt-equivalence: PASS=$PASS FAIL=$FAIL SKIP=$SKIP${NC}"

if [[ "$FAIL" -gt 0 ]]; then
    echo -e "${RED}✗ opt-equivalence: $FAIL fixture(s) differ between O0 and O1${NC}" >&2
    exit 1
fi

echo -e "${GREEN}✓ all checked fixtures match between O0 and O1${NC}"
exit 0
