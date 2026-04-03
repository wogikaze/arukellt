#!/usr/bin/env bash
# gen-harness-report.sh — Parse `cargo test -p arukellt --test harness` output
# and produce a structured JSON (or text) pass/fail report for CI artifact upload
# and regression tracking between runs.
#
# Usage:
#   bash scripts/gen/gen-harness-report.sh [OPTIONS] [LOG_FILE]
#   cargo test -p arukellt --test harness -- --nocapture 2>&1 | \
#     bash scripts/gen/gen-harness-report.sh
#
# Options:
#   LOG_FILE          Path to captured harness output (reads stdin if omitted)
#   --baseline FILE   Compare against a previous report JSON for regression tracking
#   --text            Output plain text instead of JSON
#   --help            Show this help message
#
# The ARUKELLT_BIN env var can be set before running `cargo test` to point at
# a selfhost binary; this script captures whatever binary path the harness reports.

set -euo pipefail

BASELINE=""
TEXT_OUTPUT=false
INPUT_FILE=""

usage() {
    cat <<'USAGE'
Usage: bash scripts/gen/gen-harness-report.sh [OPTIONS] [LOG_FILE]
       cargo test -p arukellt --test harness -- --nocapture 2>&1 | \
         bash scripts/gen/gen-harness-report.sh

Parse cargo test harness output and produce a structured pass/fail report.

Arguments:
  LOG_FILE          Path to captured harness output. Reads stdin if omitted.

Options:
  --baseline FILE   Compare against a previous report JSON for regression tracking.
                    Shows regressions (new failures) and recoveries (fixed failures).
  --text            Output plain text report instead of JSON.
  --help            Show this help message.

Output JSON fields:
  generated_at       ISO-8601 timestamp
  summary            pass/fail/skip counts and totals
  failures           list of { kind, path, message } for each failed fixture
  failures_by_kind   failure counts grouped by fixture kind
  arukellt_bin       binary path reported by the harness
  target             ARUKELLT_TARGET filter if set
  baseline_comparison  (only when --baseline given)
    regressions      fixtures that newly failed vs baseline
    recoveries       fixtures that newly passed vs baseline

Fixture kinds: run, diag, module-run, module-diag, t3-run, t3-compile,
               component-compile, compile-error
USAGE
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --help|-h)      usage; exit 0 ;;
        --text)         TEXT_OUTPUT=true; shift ;;
        --baseline)     BASELINE="${2:-}"; shift 2 ;;
        -*)
            echo "error: unknown option: $1" >&2
            usage >&2
            exit 1
            ;;
        *)              INPUT_FILE="$1"; shift ;;
    esac
done

# Show usage when called interactively with no input
if [ -z "$INPUT_FILE" ] && [ -t 0 ]; then
    usage
    exit 0
fi

# Read input content into env var (avoids heredoc stdin collision with python3)
if [ -n "$INPUT_FILE" ]; then
    GHR_CONTENT=$(cat "$INPUT_FILE")
else
    GHR_CONTENT=$(cat)
fi

export GHR_CONTENT
export GHR_BASELINE="$BASELINE"
export GHR_TEXT="$TEXT_OUTPUT"

python3 << 'PYTHON'
import os, json, re, datetime

content       = os.environ["GHR_CONTENT"]
baseline_file = os.environ.get("GHR_BASELINE", "")
text_output   = os.environ.get("GHR_TEXT", "false") == "true"

# ── Helpers ───────────────────────────────────────────────────────────────────

def finalise_failure(obj):
    """Merge collected detail lines into the message field."""
    if obj and "_details" in obj:
        if obj["_details"]:
            details = " | ".join(obj["_details"])
            obj["message"] = (obj["message"] + " " + details).strip()
        del obj["_details"]

# ── Parse harness output ──────────────────────────────────────────────────────

summary = {"pass": 0, "fail": 0, "skip": 0, "scheduled": 0, "total": 0}
failures = []
arukellt_bin = ""
target = ""

# Summary line printed by harness at end of run:
#   PASS: 19 FAIL: 0 SKIP: 0 (scheduled: 19, total manifest: 19)
m = re.search(
    r'PASS:\s*(\d+)\s+FAIL:\s*(\d+)\s+SKIP:\s*(\d+)\s+'
    r'\(scheduled:\s*(\d+),\s*total manifest:\s*(\d+)\)',
    content
)
if m:
    summary = {
        "pass":      int(m.group(1)),
        "fail":      int(m.group(2)),
        "skip":      int(m.group(3)),
        "scheduled": int(m.group(4)),
        "total":     int(m.group(5)),
    }

# Binary path: Using binary: "/path/to/arukellt"
m = re.search(r'Using binary:\s*"?([^"\n]+?)"?\s*$', content, re.MULTILINE)
if m:
    arukellt_bin = m.group(1).strip()

# Target filter: Target: wasm32-wasi-p2
m = re.search(r'^Target:\s*(\S+)', content, re.MULTILINE)
if m:
    target = m.group(1).strip()

# Failure blocks emitted by harness.rs:
#
#   FAIL [run] path/to/fixture.ark
#     expected: "hello\n"
#     got:      "world\n"
#
#   FAIL [compile-error] path/to/fixture.ark \u2014 expected compile failure but succeeded
#
# Each FAIL line starts a new failure; subsequent lines indented with 2+ spaces
# are detail lines until the next non-indented line.
current = None
# Regex: FAIL [kind] rest  (rest may contain em-dash note)
FAIL_RE = re.compile(r'^FAIL \[([^\]]+)\]\s+(.*)')

for line in content.splitlines():
    fm = FAIL_RE.match(line)
    if fm:
        finalise_failure(current)
        kind = fm.group(1)
        rest = fm.group(2).strip()
        # Split "path \u2014 note" or "path - note" (compile-error style)
        dash_m = re.match(r'^(.*?)\s+[\u2014\-]{1,2}\s+(.*)$', rest)
        if dash_m:
            path = dash_m.group(1).strip()
            note = dash_m.group(2).strip()
        else:
            path = rest
            note = ""
        current = {"kind": kind, "path": path, "message": note, "_details": []}
        failures.append(current)
    elif current is not None:
        if line.startswith("  ") and not line.startswith("  ["):
            current["_details"].append(line.strip())
        else:
            finalise_failure(current)
            current = None

finalise_failure(current)

# Deduplicate in case of duplicate FAIL lines in output
seen = set()
unique = []
for f in failures:
    key = (f["kind"], f["path"])
    if key not in seen:
        seen.add(key)
        unique.append(f)
failures = unique

# Count failures by fixture kind
failures_by_kind: dict = {}
for f in failures:
    failures_by_kind[f["kind"]] = failures_by_kind.get(f["kind"], 0) + 1

report = {
    "generated_at":     datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "summary":          summary,
    "failures":         failures,
    "failures_by_kind": failures_by_kind,
    "arukellt_bin":     arukellt_bin,
    "target":           target,
}

# ── Regression comparison against a baseline report ──────────────────────────

if baseline_file:
    try:
        with open(baseline_file) as fh:
            baseline = json.load(fh)
        baseline_failed = {(x["kind"], x["path"]) for x in baseline.get("failures", [])}
        current_failed  = {(x["kind"], x["path"]) for x in failures}
        regressions = [{"kind": k, "path": p} for k, p in sorted(current_failed - baseline_failed)]
        recoveries  = [{"kind": k, "path": p} for k, p in sorted(baseline_failed - current_failed)]
        report["baseline_comparison"] = {
            "baseline_generated_at": baseline.get("generated_at", ""),
            "regressions":           regressions,
            "recoveries":            recoveries,
            "regression_count":      len(regressions),
            "recovery_count":        len(recoveries),
        }
    except (FileNotFoundError, json.JSONDecodeError) as e:
        report["baseline_comparison_error"] = str(e)

# ── Output ────────────────────────────────────────────────────────────────────

if text_output:
    s = report["summary"]
    print(f"Harness Report  {report['generated_at']}")
    if report["arukellt_bin"]:
        print(f"Binary : {report['arukellt_bin']}")
    if report["target"]:
        print(f"Target : {report['target']}")
    print(f"Summary: PASS={s['pass']}  FAIL={s['fail']}  SKIP={s['skip']}"
          f"  (scheduled={s['scheduled']}, total={s['total']})")
    if failures:
        print(f"\nFailures by kind:")
        for kind, count in sorted(failures_by_kind.items()):
            print(f"  {kind}: {count}")
        print(f"\nFailed fixtures ({len(failures)}):")
        for f in failures:
            msg = f"  \u2014 {f['message']}" if f["message"] else ""
            print(f"  [{f['kind']}] {f['path']}{msg}")
    else:
        print("\nAll scheduled fixtures passed.")
    if "baseline_comparison" in report:
        bc = report["baseline_comparison"]
        print(f"\nRegression vs baseline ({bc['baseline_generated_at']}):")
        print(f"  Regressions : {bc['regression_count']}  |  Recoveries: {bc['recovery_count']}")
        for r in bc["regressions"]:
            print(f"  REGRESSED : [{r['kind']}] {r['path']}")
        for r in bc["recoveries"]:
            print(f"  RECOVERED : [{r['kind']}] {r['path']}")
else:
    print(json.dumps(report, indent=2))
PYTHON
