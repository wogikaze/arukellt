#!/usr/bin/env python3
"""
gen-harness-report.py — Parse `cargo test -p arukellt --test harness` output
and produce a structured JSON (or text) pass/fail report for CI artifact upload
and regression tracking between runs.

Usage:
  python3 scripts/gen/gen-harness-report.py [OPTIONS] [LOG_FILE]
  cargo test -p arukellt --test harness -- --nocapture 2>&1 | \
    python3 scripts/gen/gen-harness-report.py

Options:
  LOG_FILE          Path to captured harness output (reads stdin if omitted)
  --baseline FILE   Compare against a previous report JSON for regression tracking
  --text            Output plain text instead of JSON
  --help            Show this help message

The ARUKELLT_BIN env var can be set before running `cargo test` to point at
a selfhost binary; this script captures whatever binary path the harness reports.
"""

import argparse
import json
import re
import sys
from datetime import datetime, timezone


def finalise_failure(obj):
    """Merge collected detail lines into the message field."""
    if obj and "_details" in obj:
        if obj["_details"]:
            details = " | ".join(obj["_details"])
            obj["message"] = (obj["message"] + " " + details).strip()
        del obj["_details"]


def parse_harness_report(content, baseline_file=None, text_output=False):
    """Parse harness output and generate report."""
    # Parse summary line: PASS: 19 FAIL: 0 SKIP: 0 (scheduled: 19, total manifest: 19)
    summary = {"pass": 0, "fail": 0, "skip": 0, "scheduled": 0, "total": 0}
    failures = []
    arukellt_bin = ""
    target = ""

    m = re.search(
        r'PASS:\s*(\d+)\s+FAIL:\s*(\d+)\s+SKIP:\s*(\d+)\s+'
        r'\(scheduled:\s*(\d+),\s*total manifest:\s*(\d+)\)',
        content
    )
    if m:
        summary = {
            "pass": int(m.group(1)),
            "fail": int(m.group(2)),
            "skip": int(m.group(3)),
            "scheduled": int(m.group(4)),
            "total": int(m.group(5)),
        }

    # Binary path: Using binary: "/path/to/arukellt"
    m = re.search(r'Using binary:\s*"?([^"\n]+?)"?\s*$', content, re.MULTILINE)
    if m:
        arukellt_bin = m.group(1).strip()

    # Target filter: Target: wasm32-wasi-p2
    m = re.search(r'^Target:\s*(\S+)', content, re.MULTILINE)
    if m:
        target = m.group(1).strip()

    # Failure blocks:
    #   FAIL [run] path/to/fixture.ark
    #     expected: "hello\n"
    #     got:      "world\n"
    current = None
    FAIL_RE = re.compile(r'^FAIL \[([^\]]+)\]\s+(.*)')

    for line in content.splitlines():
        fm = FAIL_RE.match(line)
        if fm:
            finalise_failure(current)
            kind = fm.group(1)
            rest = fm.group(2).strip()
            # Split "path — note" or "path - note"
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

    # Deduplicate
    seen = set()
    unique = []
    for f in failures:
        key = (f["kind"], f["path"])
        if key not in seen:
            seen.add(key)
            unique.append(f)
    failures = unique

    # Count failures by kind
    failures_by_kind = {}
    for f in failures:
        failures_by_kind[f["kind"]] = failures_by_kind.get(f["kind"], 0) + 1

    report = {
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "summary": summary,
        "failures": failures,
        "failures_by_kind": failures_by_kind,
        "arukellt_bin": arukellt_bin,
        "target": target,
    }

    # Baseline comparison
    if baseline_file:
        try:
            with open(baseline_file) as fh:
                baseline = json.load(fh)
            baseline_failed = {(x["kind"], x["path"]) for x in baseline.get("failures", [])}
            current_failed = {(x["kind"], x["path"]) for x in failures}
            regressions = [{"kind": k, "path": p} for k, p in sorted(current_failed - baseline_failed)]
            recoveries = [{"kind": k, "path": p} for k, p in sorted(baseline_failed - current_failed)]
            report["baseline_comparison"] = {
                "baseline_generated_at": baseline.get("generated_at", ""),
                "regressions": regressions,
                "recoveries": recoveries,
                "regression_count": len(regressions),
                "recovery_count": len(recoveries),
            }
        except (FileNotFoundError, json.JSONDecodeError) as e:
            report["baseline_comparison_error"] = str(e)

    return report


def format_text_report(report):
    """Format report as plain text."""
    lines = []
    s = report["summary"]
    lines.append(f"Harness Report  {report['generated_at']}")
    if report["arukellt_bin"]:
        lines.append(f"Binary : {report['arukellt_bin']}")
    if report["target"]:
        lines.append(f"Target : {report['target']}")
    lines.append(f"Summary: PASS={s['pass']}  FAIL={s['fail']}  SKIP={s['skip']}"
                 f"  (scheduled={s['scheduled']}, total={s['total']})")
    
    if report["failures"]:
        lines.append("\nFailures by kind:")
        for kind, count in sorted(report["failures_by_kind"].items()):
            lines.append(f"  {kind}: {count}")
        lines.append(f"\nFailed fixtures ({len(report['failures'])}):")
        for f in report["failures"]:
            msg = f"  — {f['message']}" if f["message"] else ""
            lines.append(f"  [{f['kind']}] {f['path']}{msg}")
    else:
        lines.append("\nAll scheduled fixtures passed.")
    
    if "baseline_comparison" in report:
        bc = report["baseline_comparison"]
        lines.append(f"\nRegression vs baseline ({bc['baseline_generated_at']}):")
        lines.append(f"  Regressions : {bc['regression_count']}  |  Recoveries: {bc['recovery_count']}")
        for r in bc["regressions"]:
            lines.append(f"  REGRESSED : [{r['kind']}] {r['path']}")
        for r in bc["recoveries"]:
            lines.append(f"  RECOVERED : [{r['kind']}] {r['path']}")
    
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(
        description="Parse cargo test harness output and produce a structured pass/fail report.",
        epilog="Output JSON fields: generated_at, summary, failures, failures_by_kind, arukellt_bin, target, baseline_comparison"
    )
    parser.add_argument(
        "input_file",
        nargs="?",
        help="Path to captured harness output. Reads stdin if omitted."
    )
    parser.add_argument(
        "--baseline",
        metavar="FILE",
        help="Compare against a previous report JSON for regression tracking. Shows regressions (new failures) and recoveries (fixed failures)."
    )
    parser.add_argument(
        "--text",
        action="store_true",
        help="Output plain text report instead of JSON."
    )
    
    args = parser.parse_args()
    
    # Read input
    if args.input_file:
        with open(args.input_file) as f:
            content = f.read()
    else:
        # Show usage if called interactively with no input
        if sys.stdin.isatty():
            parser.print_help()
            sys.exit(0)
        content = sys.stdin.read()
    
    # Parse and generate report
    report = parse_harness_report(content, args.baseline, args.text)
    
    # Output
    if args.text:
        print(format_text_report(report))
    else:
        print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
