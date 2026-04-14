#!/usr/bin/env bash
# Wasm binary section size analysis tool.
#
# Usage:
#   bash scripts/run/wasm-size-analysis.sh <file.wasm> [options]
#
# Options:
#   --baseline <name>   Save results as a named baseline under benchmarks/baselines/size/
#   --diff <name>       Diff current results against a saved baseline
#   --top <N>           Show top N functions by code size (default: 10)
#   --json              Emit output as JSON
#   --help              Show this message

set -euo pipefail

RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[1;33m'
CYAN=$'\033[0;36m'
BOLD=$'\033[1m'
NC=$'\033[0m'

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BASELINE_DIR="$REPO_ROOT/benchmarks/baselines/size"

usage() {
    cat <<'EOF'
Usage: bash scripts/run/wasm-size-analysis.sh <file.wasm> [options]

  Analyze section sizes in a WebAssembly binary and optionally diff against
  a saved baseline.

Options:
  --baseline <name>   Save results as a named baseline (writes JSON)
  --diff <name>       Diff against a previously saved baseline with that name
  --top <N>           Number of top functions to show (default: 10)
  --json              Emit structured JSON output instead of human-readable text
  --help              Show this help message

Baseline files are stored under:
  benchmarks/baselines/size/<name>.json
EOF
}

# ---------- arg parse ----------
WASM_FILE=""
BASELINE_NAME=""
DIFF_NAME=""
TOP_N=10
JSON_OUTPUT=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --help|-h)
            usage; exit 0 ;;
        --baseline)
            [[ -z "${2-}" ]] && { echo "${RED}error: --baseline requires a name${NC}" >&2; exit 1; }
            BASELINE_NAME="$2"; shift 2 ;;
        --diff)
            [[ -z "${2-}" ]] && { echo "${RED}error: --diff requires a name${NC}" >&2; exit 1; }
            DIFF_NAME="$2"; shift 2 ;;
        --top)
            [[ -z "${2-}" ]] && { echo "${RED}error: --top requires a number${NC}" >&2; exit 1; }
            TOP_N="$2"; shift 2 ;;
        --json)
            JSON_OUTPUT=true; shift ;;
        -*)
            echo "${RED}error: unknown option: $1${NC}" >&2; usage; exit 1 ;;
        *)
            if [[ -z "$WASM_FILE" ]]; then
                WASM_FILE="$1"
            else
                echo "${RED}error: unexpected argument: $1${NC}" >&2; usage; exit 1
            fi
            shift ;;
    esac
done

if [[ -z "$WASM_FILE" ]]; then
    echo "${RED}error: no .wasm file specified${NC}" >&2
    usage; exit 1
fi

if [[ ! -f "$WASM_FILE" ]]; then
    echo "${RED}error: file not found: $WASM_FILE${NC}" >&2
    exit 1
fi

# ---------- dependency check ----------
if ! command -v wasm-objdump &>/dev/null; then
    echo "${RED}error: wasm-objdump not found. Install wabt (apt install wabt / brew install wabt).${NC}" >&2
    exit 1
fi

# ---------- parse section headers ----------
# wasm-objdump -h output line format:
#   <NAME> start=0x... end=0x... (size=0x...) count: N
declare -A SECTION_BYTES
declare -a SECTION_ORDER
TOTAL_SECTION_BYTES=0

while IFS= read -r line; do
    if [[ "$line" =~ ^[[:space:]]+([A-Za-z]+)[[:space:]]+start=0x[0-9a-f]+[[:space:]]+end=0x[0-9a-f]+[[:space:]]+\(size=(0x[0-9a-f]+)\) ]]; then
        sec_name="${BASH_REMATCH[1]}"
        sec_size_hex="${BASH_REMATCH[2]}"
        sec_size=$((sec_size_hex))   # bash arithmetic converts hex
        sec_name_lower="${sec_name,,}"
        SECTION_BYTES["$sec_name_lower"]=$sec_size
        SECTION_ORDER+=("$sec_name_lower")
        TOTAL_SECTION_BYTES=$((TOTAL_SECTION_BYTES + sec_size))
    fi
done < <(wasm-objdump -h "$WASM_FILE" 2>/dev/null)

FILE_BYTES=$(wc -c < "$WASM_FILE")

# ---------- parse function sizes from code section ----------
declare -A FUNC_SIZES
declare -a FUNC_ORDER

# Store pattern in variable so bash does not interpret '<(' as process substitution
_func_re='^[[:space:]]+-[[:space:]]+func\[([0-9]+)\][[:space:]]+size=([0-9]+)([[:space:]]+<([^>]*)>)?'

while IFS= read -r line; do
    # Line format: " - func[N] size=K <name>" or " - func[N] size=K"
    if [[ "$line" =~ $_func_re ]]; then
        func_idx="${BASH_REMATCH[1]}"
        func_size="${BASH_REMATCH[2]}"
        func_name="${BASH_REMATCH[4]:-}"
        [[ -z "$func_name" ]] && func_name="func[$func_idx]"
        FUNC_SIZES["$func_idx:$func_name"]=$func_size
        FUNC_ORDER+=("$func_idx:$func_name")
    fi
done < <(wasm-objdump -x "$WASM_FILE" 2>/dev/null | grep -A10000 '^Code\[')

# ---------- build sorted top-N list ----------
declare -a SORTED_FUNCS
# Sort by size descending using process substitution
while IFS=$'\t' read -r size key; do
    SORTED_FUNCS+=("$size:$key")
done < <(
    for key in "${!FUNC_SIZES[@]}"; do
        echo -e "${FUNC_SIZES[$key]}\t$key"
    done | sort -rn
)

# ---------- JSON output ----------
if [[ "$JSON_OUTPUT" == true ]]; then
    # emit JSON
    echo "{"
    echo "  \"file\": \"$WASM_FILE\","
    echo "  \"total_bytes\": $FILE_BYTES,"
    echo "  \"sections\": {"
    first=true
    for sec in "${SECTION_ORDER[@]}"; do
        [[ "$first" == true ]] && first=false || echo ","
        printf '    "%s": %d' "$sec" "${SECTION_BYTES[$sec]}"
    done
    echo ""
    echo "  },"
    echo "  \"top_functions\": ["
    count=0
    first=true
    for entry in "${SORTED_FUNCS[@]}"; do
        [[ $count -ge $TOP_N ]] && break
        size="${entry%%:*}"
        rest="${entry#*:}"
        idx="${rest%%:*}"
        name="${rest#*:}"
        [[ "$first" == true ]] && first=false || echo ","
        printf '    {"index": %d, "name": "%s", "bytes": %d}' "$idx" "$name" "$size"
        count=$((count + 1))
    done
    echo ""
    echo "  ]"
    echo "}"
    exit 0
fi

# ---------- human-readable output ----------
echo ""
echo "${BOLD}Wasm size analysis: $(basename "$WASM_FILE")${NC}"
echo "Total file size: ${FILE_BYTES} bytes"
echo ""

# Sections table
echo "${CYAN}Sections:${NC}"
for sec in "${SECTION_ORDER[@]}"; do
    bytes="${SECTION_BYTES[$sec]}"
    if [[ $TOTAL_SECTION_BYTES -gt 0 ]]; then
        pct=$(( bytes * 100 / TOTAL_SECTION_BYTES ))
    else
        pct=0
    fi
    # Human-readable size
    if [[ $bytes -ge 1024 ]]; then
        hr=$(awk "BEGIN { printf \"%.1fKB\", $bytes/1024 }")
    else
        hr="${bytes}B"
    fi
    printf "  %-12s %8s  (%3d%%)\n" "${sec}" "$hr" "$pct"
done
echo "  ─────────────────────────"
if [[ $TOTAL_SECTION_BYTES -ge 1024 ]]; then
    total_hr=$(awk "BEGIN { printf \"%.1fKB\", $TOTAL_SECTION_BYTES/1024 }")
else
    total_hr="${TOTAL_SECTION_BYTES}B"
fi
printf "  %-12s %8s\n" "total" "$total_hr"

# Top functions
if [[ ${#SORTED_FUNCS[@]} -gt 0 ]]; then
    echo ""
    echo "${CYAN}Top ${TOP_N} functions by code size:${NC}"
    count=0
    code_bytes="${SECTION_BYTES[code]:-0}"
    for entry in "${SORTED_FUNCS[@]}"; do
        [[ $count -ge $TOP_N ]] && break
        size="${entry%%:*}"
        rest="${entry#*:}"
        idx="${rest%%:*}"
        name="${rest#*:}"
        if [[ $code_bytes -gt 0 ]]; then
            pct=$(( size * 100 / code_bytes ))
        else
            pct=0
        fi
        if [[ $size -ge 1024 ]]; then
            hr=$(awk "BEGIN { printf \"%.1fKB\", $size/1024 }")
        else
            hr="${size}B"
        fi
        printf "  %-32s %8s  (%3d%%)\n" "$name" "$hr" "$pct"
        count=$((count + 1))
    done
fi

# ---------- save baseline ----------
if [[ -n "$BASELINE_NAME" ]]; then
    mkdir -p "$BASELINE_DIR"
    baseline_file="$BASELINE_DIR/${BASELINE_NAME}.json"

    # Build JSON for baseline
    {
        echo "{"
        echo "  \"name\": \"$BASELINE_NAME\","
        echo "  \"file\": \"$(basename "$WASM_FILE")\","
        echo "  \"total_bytes\": $FILE_BYTES,"
        echo "  \"sections\": {"
        first=true
        for sec in "${SECTION_ORDER[@]}"; do
            [[ "$first" == true ]] && first=false || echo ","
            printf '    "%s": %d' "$sec" "${SECTION_BYTES[$sec]}"
        done
        echo ""
        echo "  },"
        echo "  \"top_functions\": ["
        count=0
        first=true
        for entry in "${SORTED_FUNCS[@]}"; do
            [[ $count -ge $TOP_N ]] && break
            size="${entry%%:*}"
            rest="${entry#*:}"
            idx="${rest%%:*}"
            name="${rest#*:}"
            [[ "$first" == true ]] && first=false || echo ","
            printf '    {"index": %d, "name": "%s", "bytes": %d}' "$idx" "$name" "$size"
            count=$((count + 1))
        done
        echo ""
        echo "  ]"
        echo "}"
    } > "$baseline_file"

    echo ""
    echo "${GREEN}Baseline saved: $baseline_file${NC}"
fi

# ---------- diff against baseline ----------
if [[ -n "$DIFF_NAME" ]]; then
    baseline_file="$BASELINE_DIR/${DIFF_NAME}.json"
    if [[ ! -f "$baseline_file" ]]; then
        echo "${RED}error: baseline not found: $baseline_file${NC}" >&2
        exit 1
    fi

    echo ""
    echo "${CYAN}Diff against baseline '${DIFF_NAME}':${NC}"

    # Read baseline total_bytes
    base_total=$(grep '"total_bytes"' "$baseline_file" | grep -o '[0-9]*' | head -1)
    diff_total=$((FILE_BYTES - base_total))
    if [[ $diff_total -gt 0 ]]; then
        color="$RED"
        sign="+"
    elif [[ $diff_total -lt 0 ]]; then
        color="$GREEN"
        sign=""
    else
        color="$NC"
        sign=""
    fi
    printf "  %-20s %8d → %8d  (%s%+d bytes%s)\n" \
        "total file size" "$base_total" "$FILE_BYTES" "$color" "$diff_total" "$NC"

    # Section diffs
    for sec in "${SECTION_ORDER[@]}"; do
        cur="${SECTION_BYTES[$sec]}"
        base=$(grep "\"$sec\"" "$baseline_file" | grep -o '[0-9]*' | head -1 || echo 0)
        [[ -z "$base" ]] && base=0
        d=$((cur - base))
        if [[ $d -ne 0 ]]; then
            printf "  %-20s %8d → %8d  (%+d bytes)\n" "$sec" "$base" "$cur" "$d"
        fi
    done
fi

echo ""
