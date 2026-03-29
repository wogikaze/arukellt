#!/usr/bin/env bash
# bench-trend.sh — show benchmark trends over the last N result files.
#
# Usage:
#   bash scripts/bench-trend.sh           # last 10 results
#   bash scripts/bench-trend.sh -n 5      # last 5 results
#   bash scripts/bench-trend.sh -n 20     # last 20 results
#
# Reads all JSON results from benchmarks/results/ and displays a text table
# with sparkline-style indicators showing metric trends.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RESULTS_DIR="$ROOT/benchmarks/results"
COUNT=10

# --- arg parsing -------------------------------------------------------------
while [[ $# -gt 0 ]]; do
  case "$1" in
    -n|--count) COUNT="$2"; shift 2 ;;
    -h|--help)
      echo "Usage: bench-trend.sh [-n COUNT]"
      echo "  Show benchmark trends over the last N result files (default: 10)"
      exit 0
      ;;
    *) echo "Unknown flag: $1" >&2; exit 1 ;;
  esac
done

# --- collect result files sorted by name (date-prefixed) ---------------------
RESULT_FILES=()
while IFS= read -r f; do
  RESULT_FILES+=("$f")
done < <(find "$RESULTS_DIR" -maxdepth 1 -name '*.json' \
  ! -name 'latest.json' ! -name '.*' | sort)

if [[ ${#RESULT_FILES[@]} -eq 0 ]]; then
  echo "No result files found in benchmarks/results/" >&2
  exit 1
fi

# Pass all files to Python for trend analysis
python3 - "$COUNT" "${RESULT_FILES[@]}" <<'PYEOF'
import json, sys, os

BOLD   = "\033[1m"
DIM    = "\033[2m"
RED    = "\033[31m"
GREEN  = "\033[32m"
YELLOW = "\033[33m"
RESET  = "\033[0m"

if not sys.stdout.isatty():
    BOLD = DIM = RED = GREEN = YELLOW = RESET = ""

count = int(sys.argv[1])
files = sys.argv[2:]

# Take last N files
files = files[-count:]

# Load all results
results = []
for path in files:
    try:
        with open(path) as f:
            data = json.load(f)
        data["_file"] = os.path.basename(path)
        results.append(data)
    except (json.JSONDecodeError, OSError):
        pass

if not results:
    print("No valid result files found.", file=sys.stderr)
    sys.exit(1)

# Sparkline characters (8 levels)
SPARKS = "▁▂▃▄▅▆▇█"

def sparkline(values):
    """Generate a sparkline string for a list of numeric values."""
    if not values or all(v is None for v in values):
        return "—"
    clean = [v for v in values if v is not None]
    if not clean:
        return "—"
    lo, hi = min(clean), max(clean)
    span = hi - lo if hi != lo else 1.0
    chars = []
    for v in values:
        if v is None:
            chars.append("·")
        else:
            idx = int((v - lo) / span * (len(SPARKS) - 1))
            idx = max(0, min(idx, len(SPARKS) - 1))
            chars.append(SPARKS[idx])
    return "".join(chars)

def trend_arrow(values):
    """Return a colored trend indicator comparing last value to median."""
    clean = [v for v in values if v is not None]
    if len(clean) < 2:
        return "  —"
    last = clean[-1]
    # Median of all but last
    prev = sorted(clean[:-1])
    median = prev[len(prev) // 2]
    if median == 0:
        return "  —"
    pct = ((last - median) / median) * 100.0
    if pct > 5:
        return f"{RED}▲{pct:+.0f}%{RESET}"
    elif pct < -5:
        return f"{GREEN}▼{pct:+.0f}%{RESET}"
    else:
        return f"  ≈{pct:+.0f}%"

# Gather all benchmark names across all runs
all_names = []
seen = set()
for r in results:
    for b in r.get("benchmarks", []):
        if b["name"] not in seen:
            all_names.append(b["name"])
            seen.add(b["name"])

# Print header with run labels
print(f"{BOLD}Benchmark Trends (last {len(results)} runs){RESET}")
print()

# Show run summary
print(f"{BOLD}Runs:{RESET}")
run_hdr = "  {:<4s} {:<28s} {:>10s}  {:<7s}"
print(run_hdr.format("#", "File", "Date", "Commit"))
print(run_hdr.format("─" * 4, "─" * 28, "─" * 10, "─" * 7))
for i, r in enumerate(results):
    fname = r["_file"]
    generated = r.get("generated_at", "—")[:10]
    commit = r.get("commit_short", fname.split("-")[1].replace(".json", "")[:7] if "-" in fname else "—")
    print(run_hdr.format(f"[{i}]", fname, generated, commit))
print()

# Metric sections
sections = [
    ("Compile Time (ms)", "compile", "median_ms", False),
    ("Runtime (ms)",      "runtime", "median_ms", False),
    ("Binary Size (B)",   "compile", "binary_bytes", True),
]

for title, phase, metric, is_int in sections:
    print(f"{BOLD}── {title} ──{RESET}")

    hdr = "{:<20s} {:>10s} {:>10s} {:>10s}  {:<15s} {:>8s}"
    print(hdr.format("Benchmark", "First", "Last", "Median", "Sparkline", "Trend"))
    print(hdr.format("─" * 20, "─" * 10, "─" * 10, "─" * 10, "─" * 15, "─" * 8))

    for name in all_names:
        values = []
        for r in results:
            bench = next((b for b in r.get("benchmarks", []) if b["name"] == name), None)
            if bench is None:
                values.append(None)
                continue
            p = bench.get(phase, {})
            if p.get("status") == "skipped":
                values.append(None)
                continue
            values.append(p.get(metric))

        clean = [v for v in values if v is not None]
        if not clean:
            print(hdr.format(name, "—", "—", "—", "—", "—"))
            continue

        first_val = clean[0]
        last_val = clean[-1]
        median_val = sorted(clean)[len(clean) // 2]

        if is_int:
            first_s = f"{int(first_val)}"
            last_s  = f"{int(last_val)}"
            med_s   = f"{int(median_val)}"
        else:
            first_s = f"{first_val:.1f}"
            last_s  = f"{last_val:.1f}"
            med_s   = f"{median_val:.1f}"

        spark = sparkline(values)
        trend = trend_arrow(values)

        print(hdr.format(name, first_s, last_s, med_s, spark, trend))

    print()

print(f"{DIM}Sparkline: ▁=low ███=high · =missing  Trend: vs median of prior runs{RESET}")
PYEOF
