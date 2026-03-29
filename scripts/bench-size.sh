#!/usr/bin/env bash
# bench-size.sh — Wasm binary size attribution and diff tracking.
# Compiles benchmark .ark files to .wasm, reports per-section sizes,
# and compares against a stored baseline when available.
#
# Usage:
#   scripts/bench-size.sh                  # report + diff (if baseline exists)
#   scripts/bench-size.sh --update         # report + overwrite baseline
#   scripts/bench-size.sh --json           # emit JSON to stdout only
#
# Environment:
#   ARUKELLT  — path to compiler binary (default: target/release/arukellt)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ARUKELLT="${ARUKELLT:-$REPO_ROOT/target/release/arukellt}"
BASELINE="$REPO_ROOT/benchmarks/baselines/sizes.json"
BUILD_DIR="$REPO_ROOT/target/bench-size"
UPDATE_BASELINE=false
JSON_ONLY=false

for arg in "$@"; do
  case "$arg" in
    --update) UPDATE_BASELINE=true ;;
    --json)   JSON_ONLY=true ;;
    --help|-h)
      sed -n '2,/^[^#]/{ /^#/s/^# \?//p }' "$0"
      exit 0
      ;;
  esac
done

if ! command -v "$ARUKELLT" &>/dev/null && [[ ! -x "$ARUKELLT" ]]; then
  echo "error: arukellt compiler not found at $ARUKELLT" >&2
  echo "       run 'cargo build --release' first" >&2
  exit 1
fi

HAS_WASM_TOOLS=false
if command -v wasm-tools &>/dev/null; then
  HAS_WASM_TOOLS=true
fi

mkdir -p "$BUILD_DIR"

# --- helpers ----------------------------------------------------------------

file_size() {
  stat -c %s "$1" 2>/dev/null || stat -f %z "$1"
}

# Parse wasm-tools objdump output into JSON section map.
# Reads lines like:
#   code       | 0xdf - 0x2c2 | 483 bytes | 9 count
sections_json() {
  local wasm="$1"
  if [[ "$HAS_WASM_TOOLS" == true ]]; then
    wasm-tools objdump "$wasm" 2>/dev/null | awk '
      BEGIN { printf "{" ; first=1 }
      /\|.*bytes/ {
        gsub(/^[ \t]+|[ \t]+$/, "", $1)
        name = $1
        # find the bytes value: field after second |
        for (i=1; i<=NF; i++) {
          if ($i == "bytes") { size = $(i-1); break }
        }
        if (!first) printf ","
        printf "\"%s\":%s", name, size
        first = 0
      }
      END { printf "}" }
    '
  else
    # Fallback: use arukellt analyze (produces human text; scrape it)
    "$ARUKELLT" analyze --wasm-size "$wasm" 2>/dev/null | awk '
      BEGIN { printf "{"; first=1 }
      /^[a-z]+: [0-9]+ bytes/ {
        name = $1; sub(/:$/, "", name)
        size = $2
        if (!first) printf ","
        printf "\"%s\":%s", name, size
        first = 0
      }
      END { printf "}" }
    '
  fi
}

# Top functions by code size (from arukellt analyze).
top_funcs_json() {
  local wasm="$1"
  "$ARUKELLT" analyze --wasm-size "$wasm" 2>/dev/null | awk '
    BEGIN { printf "["; first=1 }
    /^  func\[/ {
      gsub(/func\[/, "", $1); gsub(/\]:/, "", $1)
      idx = $1; size = $2
      if (!first) printf ","
      printf "{\"index\":%s,\"bytes\":%s}", idx, size
      first = 0
    }
    END { printf "]" }
  '
}

# --- main collection --------------------------------------------------------

RESULTS="["
first_result=true

for ark in "$REPO_ROOT"/benchmarks/*.ark; do
  name="$(basename "$ark" .ark)"
  out="$BUILD_DIR/${name}.wasm"

  if ! "$ARUKELLT" compile --target wasm32-wasi-p1 -o "$out" "$ark" &>/dev/null; then
    echo "warn: failed to compile $name, skipping" >&2
    continue
  fi

  total=$(file_size "$out")
  sections=$(sections_json "$out")
  top_funcs=$(top_funcs_json "$out")

  entry=$(cat <<EOF
{"name":"${name}","total_bytes":${total},"sections":${sections},"top_functions":${top_funcs}}
EOF
)

  if [[ "$first_result" == true ]]; then
    first_result=false
  else
    RESULTS+=","
  fi
  RESULTS+="$entry"
done

RESULTS+="]"

# --- diff against baseline --------------------------------------------------

generate_diff() {
  local current="$1" baseline="$2"
  python3 -c "
import json, sys

current = json.loads(sys.argv[1])
baseline = json.loads(open(sys.argv[2]).read())

bl = {b['name']: b for b in baseline}
diffs = []
for c in current:
    name = c['name']
    b = bl.get(name)
    if not b:
        diffs.append({'name': name, 'status': 'new', 'total_bytes': c['total_bytes']})
        continue
    d = {'name': name, 'total_delta': c['total_bytes'] - b['total_bytes']}
    sec_delta = {}
    for k, v in c.get('sections', {}).items():
        old = b.get('sections', {}).get(k, 0)
        if v - old != 0:
            sec_delta[k] = v - old
    for k, v in b.get('sections', {}).items():
        if k not in c.get('sections', {}):
            sec_delta[k] = -v
    d['section_deltas'] = sec_delta
    diffs.append(d)
for b_name in bl:
    if not any(c['name'] == b_name for c in current):
        diffs.append({'name': b_name, 'status': 'removed'})
print(json.dumps(diffs, indent=2))
" "$current" "$baseline"
}

DIFF_JSON="null"
if [[ -f "$BASELINE" ]]; then
  DIFF_JSON=$(generate_diff "$RESULTS" "$BASELINE")
fi

# --- output ------------------------------------------------------------------

FULL_JSON=$(python3 -c "
import json, sys
results = json.loads(sys.argv[1])
diff = json.loads(sys.argv[2])
out = {'benchmarks': results}
if diff is not None:
    out['diff_vs_baseline'] = diff
print(json.dumps(out, indent=2))
" "$RESULTS" "$DIFF_JSON")

if [[ "$JSON_ONLY" == true ]]; then
  echo "$FULL_JSON"
else
  echo "=== Wasm Size Attribution ==="
  echo ""
  python3 -c "
import json, sys
data = json.loads(sys.argv[1])
for b in data['benchmarks']:
    name = b['name']
    total = b['total_bytes']
    print(f'  {name}: {total} bytes')
    for sec, sz in sorted(b.get('sections', {}).items(), key=lambda x: -x[1]):
        pct = sz / total * 100 if total else 0
        print(f'    {sec:16s} {sz:6d} B  ({pct:5.1f}%)')
    top = b.get('top_functions', [])
    if top:
        print('    top functions:')
        for f in top[:5]:
            print(f'      func[{f[\"index\"]}]: {f[\"bytes\"]} bytes')
    print()
if 'diff_vs_baseline' in data:
    print('=== Diff vs Baseline ===')
    print()
    for d in data['diff_vs_baseline']:
        name = d['name']
        if d.get('status') == 'new':
            print(f'  {name}: NEW ({d[\"total_bytes\"]} bytes)')
            continue
        if d.get('status') == 'removed':
            print(f'  {name}: REMOVED')
            continue
        delta = d.get('total_delta', 0)
        sign = '+' if delta >= 0 else ''
        print(f'  {name}: {sign}{delta} bytes')
        for sec, sd in sorted(d.get('section_deltas', {}).items(), key=lambda x: -abs(x[1])):
            s = '+' if sd >= 0 else ''
            print(f'    {sec:16s} {s}{sd} B')
    print()
" "$FULL_JSON"

  echo "$FULL_JSON"
fi

# --- update baseline ---------------------------------------------------------

if [[ "$UPDATE_BASELINE" == true ]]; then
  python3 -c "
import json, sys
print(json.dumps(json.loads(sys.argv[1]), indent=2))
" "$RESULTS" > "$BASELINE"
  echo ""
  echo "Baseline updated: $BASELINE"
fi
