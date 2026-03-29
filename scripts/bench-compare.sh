#!/usr/bin/env bash
# bench-compare.sh — compare two benchmark result files and show deltas.
#
# Usage:
#   bash scripts/bench-compare.sh <baseline.json> <current.json>
#   bash scripts/bench-compare.sh               # compares 2nd-latest vs latest
#
# Highlights regressions (>5% worse) in red, improvements (>5% better) in green.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RESULTS_DIR="$ROOT/benchmarks/results"
REGRESSION_THRESHOLD=5  # percent

# --- resolve input files -----------------------------------------------------
if [[ $# -ge 2 ]]; then
  BASELINE="$1"
  CURRENT="$2"
elif [[ $# -eq 0 ]]; then
  # Auto-detect: compare the two most recent result files
  RESULT_FILES=()
  while IFS= read -r f; do
    RESULT_FILES+=("$f")
  done < <(find "$RESULTS_DIR" -maxdepth 1 -name '*.json' \
    ! -name 'latest.json' ! -name '.*' | sort)
  if [[ ${#RESULT_FILES[@]} -lt 2 ]]; then
    echo "ERROR: need at least 2 result files in benchmarks/results/" >&2
    exit 1
  fi
  BASELINE="${RESULT_FILES[-2]}"
  CURRENT="${RESULT_FILES[-1]}"
else
  echo "Usage: bench-compare.sh [<baseline.json> <current.json>]" >&2
  exit 1
fi

if [[ ! -f "$BASELINE" ]]; then echo "ERROR: not found: $BASELINE" >&2; exit 1; fi
if [[ ! -f "$CURRENT" ]];  then echo "ERROR: not found: $CURRENT" >&2;  exit 1; fi

# --- comparison via Python ---------------------------------------------------
python3 - "$BASELINE" "$CURRENT" "$REGRESSION_THRESHOLD" <<'PYEOF'
import json, sys, os

RED    = "\033[31m"
GREEN  = "\033[32m"
YELLOW = "\033[33m"
BOLD   = "\033[1m"
RESET  = "\033[0m"

# Disable colors if not a terminal
if not sys.stdout.isatty():
    RED = GREEN = YELLOW = BOLD = RESET = ""

baseline_path = sys.argv[1]
current_path  = sys.argv[2]
threshold     = float(sys.argv[3])

with open(baseline_path) as f:
    baseline = json.load(f)
with open(current_path) as f:
    current = json.load(f)

base_by_name = {b["name"]: b for b in baseline.get("benchmarks", [])}
curr_by_name = {b["name"]: b for b in current.get("benchmarks", [])}

all_names = sorted(set(list(base_by_name.keys()) + list(curr_by_name.keys())))

base_label = os.path.basename(baseline_path)
curr_label = os.path.basename(current_path)

print(f"{BOLD}Benchmark Comparison{RESET}")
print(f"  Baseline: {base_label}")
print(f"  Current:  {curr_label}")
print(f"  Threshold: ±{threshold:.0f}%")
print()

# Table header
hdr_fmt = "{:<20s} {:>12s} {:>12s} {:>10s} {:>8s}"
row_fmt = "{:<20s} {:>12s} {:>12s} {:>10s} {:>8s}"

sections = [
    ("Compile Time (ms)", "compile", "median_ms"),
    ("Runtime (ms)",      "runtime", "median_ms"),
    ("Binary Size (B)",   "compile", "binary_bytes"),
]

regressions = 0
improvements = 0

for section_title, phase, metric in sections:
    print(f"{BOLD}── {section_title} ──{RESET}")
    print(hdr_fmt.format("Benchmark", "Baseline", "Current", "Delta", "Status"))
    print(hdr_fmt.format("─" * 20, "─" * 12, "─" * 12, "─" * 10, "─" * 8))

    for name in all_names:
        base_bench = base_by_name.get(name)
        curr_bench = curr_by_name.get(name)

        if not base_bench or not curr_bench:
            status = "n/a"
            print(row_fmt.format(name, "—", "—", "—", status))
            continue

        base_phase = base_bench.get(phase, {})
        curr_phase = curr_bench.get(phase, {})

        if base_phase.get("status") == "skipped" or curr_phase.get("status") == "skipped":
            print(row_fmt.format(name, "skip", "skip", "—", "skip"))
            continue

        base_val = base_phase.get(metric)
        curr_val = curr_phase.get(metric)

        if base_val is None or curr_val is None:
            print(row_fmt.format(name, "—", "—", "—", "n/a"))
            continue

        base_val = float(base_val)
        curr_val = float(curr_val)

        if base_val == 0:
            delta_pct = 0.0
        else:
            delta_pct = ((curr_val - base_val) / base_val) * 100.0

        delta_abs = curr_val - base_val

        # Format values
        if metric == "binary_bytes":
            base_str = f"{int(base_val)}"
            curr_str = f"{int(curr_val)}"
            delta_str = f"{int(delta_abs):+d} ({delta_pct:+.1f}%)"
        else:
            base_str = f"{base_val:.1f}"
            curr_str = f"{curr_val:.1f}"
            delta_str = f"{delta_abs:+.1f} ({delta_pct:+.1f}%)"

        # Status with color
        if delta_pct > threshold:
            status = f"{RED}▲ REGR{RESET}"
            regressions += 1
        elif delta_pct < -threshold:
            status = f"{GREEN}▼ IMPR{RESET}"
            improvements += 1
        else:
            status = "  ≈"

        print(row_fmt.format(name, base_str, curr_str, delta_str, status))

    print()

# Summary
print(f"{BOLD}Summary:{RESET} {regressions} regression(s), {improvements} improvement(s)")
if regressions > 0:
    print(f"{RED}⚠  Regressions detected (>{threshold:.0f}% worse){RESET}")
    sys.exit(1)
else:
    print(f"{GREEN}✓  No regressions{RESET}")
PYEOF
