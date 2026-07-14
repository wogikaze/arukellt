#!/usr/bin/env python3
"""Write a machine-readable verify-full receipt from captured output.

This script parses the stdout of `python3 scripts/manager.py verify full`
and writes a structured JSON receipt with per-check and per-item identity.

The receipt schema separates aggregate checks from individual items:
- aggregate_checks: summary-level pass/fail/skip counts per domain
- items: individual fixture/test IDs with result, category, owner_issue

Usage:
    python3 scripts/manager.py verify full 2>&1 | \
    python3 scripts/gen/write-verify-receipt.py --output docs/data/verify-full-receipt.json

Or via pipe:
    python3 scripts/gen/write-verify-receipt.py --input <file> --output <file>
"""

import argparse
import datetime
import json
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
ANSI_RE = re.compile(r"\x1b\[[0-9;]*m")

# Owner issue mapping per domain
OWNER_ISSUES = {
    "fixture_parity": "807",
    "t3_wasm_validate": "808",
    "wat_roundtrip": "809",
    "component_interop": "810",
    "cli_parity": "811",
    "diag_parity": "812",
    "fixpoint": "813",
    "formatter_parser": "814",
    "skip_debt": "815",
}


def strip_ansi(s: str) -> str:
    return ANSI_RE.sub("", s)


def get_git_commit() -> str:
    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            capture_output=True, text=True, cwd=REPO_ROOT, timeout=10,
        )
        return result.stdout.strip()
    except Exception:
        return "unknown"


def parse_receipt(text: str) -> dict:
    lines = text.splitlines()
    clean_lines = [strip_ansi(l) for l in lines]

    receipt = {
        "schema_version": 2,
        "verified_commit": get_git_commit(),
        "started_at": None,
        "finished_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "exit_status": None,
        "command": "python3 scripts/manager.py verify full",
        "aggregate_checks": [],
        "items": [],
    }

    # ── Parse aggregate domain results ──────────────────────────────
    for line in clean_lines:
        # fixture-parity: PASS=804 FAIL=367 SKIP=417
        m = re.search(r"fixture-parity: PASS=(\d+) FAIL=(\d+) SKIP=(\d+)", line)
        if m:
            receipt["aggregate_checks"].append({
                "check_id": "fixture_parity",
                "category": "fixture-parity",
                "result": "fail" if int(m.group(2)) > 0 else "pass",
                "pass_count": int(m.group(1)),
                "fail_count": int(m.group(2)),
                "skip_count": int(m.group(3)),
                "owner_issue": OWNER_ISSUES["fixture_parity"],
            })
            continue

        # cli-parity: PASS=17 FAIL=2
        m = re.search(r"cli-parity: PASS=(\d+) FAIL=(\d+)", line)
        if m:
            receipt["aggregate_checks"].append({
                "check_id": "cli_parity",
                "category": "cli-parity",
                "result": "fail" if int(m.group(2)) > 0 else "pass",
                "pass_count": int(m.group(1)),
                "fail_count": int(m.group(2)),
                "skip_count": 0,
                "owner_issue": OWNER_ISSUES["cli_parity"],
            })
            continue

        # diag-parity: PASS=29 SKIP=26 FAIL=3
        m = re.search(r"diag-parity: PASS=(\d+) SKIP=(\d+) FAIL=(\d+)", line)
        if m:
            receipt["aggregate_checks"].append({
                "check_id": "diag_parity",
                "category": "diag-parity",
                "result": "fail" if int(m.group(3)) > 0 else "pass",
                "pass_count": int(m.group(1)),
                "fail_count": int(m.group(3)),
                "skip_count": int(m.group(2)),
                "owner_issue": OWNER_ISSUES["diag_parity"],
            })
            continue

        # WAT roundtrip summary
        if "WAT roundtrip" in line and "✗" in line:
            receipt["aggregate_checks"].append({
                "check_id": "wat_roundtrip",
                "category": "target-contract",
                "result": "fail",
                "pass_count": 0,
                "fail_count": 1,
                "skip_count": 0,
                "owner_issue": OWNER_ISSUES["wat_roundtrip"],
            })
            continue

        if "WAT roundtrip" in line and "✓" in line:
            receipt["aggregate_checks"].append({
                "check_id": "wat_roundtrip",
                "category": "target-contract",
                "result": "pass",
                "pass_count": 1,
                "fail_count": 0,
                "skip_count": 0,
                "owner_issue": OWNER_ISSUES["wat_roundtrip"],
            })
            continue

        # component interop summary
        if "component interop" in line and "✗" in line and "summary" not in line.lower():
            # Individual component failures are parsed as items below
            pass

        # selfhost fixpoint
        if "selfhost fixpoint not reached" in line:
            receipt["aggregate_checks"].append({
                "check_id": "fixpoint",
                "category": "bootstrap",
                "result": "fail",
                "pass_count": 0,
                "fail_count": 1,
                "skip_count": 0,
                "owner_issue": OWNER_ISSUES["fixpoint"],
            })
            continue

        if "selfhost fixpoint reached" in line:
            receipt["aggregate_checks"].append({
                "check_id": "fixpoint",
                "category": "bootstrap",
                "result": "pass",
                "pass_count": 1,
                "fail_count": 0,
                "skip_count": 0,
                "owner_issue": OWNER_ISSUES["fixpoint"],
            })
            continue

        # T3 fixture WASM validation gate
        if "T3 fixture WASM validation gate" in line and "✗" in line:
            receipt["aggregate_checks"].append({
                "check_id": "t3_wasm_validate",
                "category": "wasm-validation",
                "result": "fail",
                "pass_count": 0,
                "fail_count": 1,
                "skip_count": 0,
                "owner_issue": OWNER_ISSUES["t3_wasm_validate"],
            })
            continue

        if "T3 fixture WASM validation gate" in line and "✓" in line:
            receipt["aggregate_checks"].append({
                "check_id": "t3_wasm_validate",
                "category": "wasm-validation",
                "result": "pass",
                "pass_count": 1,
                "fail_count": 0,
                "skip_count": 0,
                "owner_issue": OWNER_ISSUES["t3_wasm_validate"],
            })
            continue

        # Quick check summary
        m = re.search(r"Total checks: (\d+)", line)
        if m:
            # Look for the next lines with Passed/Failed/Skipped
            idx = clean_lines.index(line)
            passed = failed = skipped = 0
            for j in range(idx + 1, min(idx + 4, len(clean_lines))):
                m2 = re.search(r"Passed: (\d+)", clean_lines[j])
                if m2:
                    passed = int(m2.group(1))
                m2 = re.search(r"Failed: (\d+)", clean_lines[j])
                if m2:
                    failed = int(m2.group(1))
                m2 = re.search(r"Skipped: (\d+)", clean_lines[j])
                if m2:
                    skipped = int(m2.group(1))
            receipt["aggregate_checks"].append({
                "check_id": "quick_checks",
                "category": "quality-gate",
                "result": "fail" if failed > 0 else "pass",
                "pass_count": passed,
                "fail_count": failed,
                "skip_count": skipped,
                "owner_issue": None,
            })
            continue

    # ── Parse individual fixture items ──────────────────────────────
    # Fixture parity failures and skips
    for line in clean_lines:
        m = re.match(r"  FAIL: (\S+)", line)
        if m:
            receipt["items"].append({
                "check_id": "fixture_parity",
                "item_id": m.group(1),
                "result": "fail",
                "category": "fixture-parity",
                "owner_issue": OWNER_ISSUES["fixture_parity"],
                "baseline_status": "existing",
                "new_or_existing": "existing",
            })
            continue
        m = re.match(r"  skip: (\S+)", line)
        if m:
            receipt["items"].append({
                "check_id": "fixture_parity",
                "item_id": m.group(1),
                "result": "skip",
                "category": "fixture-parity",
                "owner_issue": OWNER_ISSUES["skip_debt"],
                "baseline_status": "existing",
                "new_or_existing": "existing",
            })
            continue

    # Component interop failures
    component_items = []
    seen_component: set[str] = set()
    for line in clean_lines:
        m = re.match(r"✗ component interop: (\S+)", line)
        if m and m.group(1) not in seen_component:
            seen_component.add(m.group(1))
            component_items.append({
                "check_id": "component_interop",
                "item_id": m.group(1),
                "result": "fail",
                "category": "component-interop",
                "owner_issue": OWNER_ISSUES["component_interop"],
                "baseline_status": "existing",
                "new_or_existing": "existing",
            })
    receipt["items"].extend(component_items)
    if component_items:
        receipt["aggregate_checks"].append({
            "check_id": "component_interop",
            "category": "component-interop",
            "result": "fail",
            "pass_count": 0,
            "fail_count": len(component_items),
            "skip_count": 0,
            "owner_issue": OWNER_ISSUES["component_interop"],
        })

    # CLI parity failures
    for line in clean_lines:
        m = re.match(r"  FAIL: (.+?) \(drifts from golden", line)
        if m:
            receipt["items"].append({
                "check_id": "cli_parity",
                "item_id": m.group(1).strip(),
                "result": "fail",
                "category": "cli-parity",
                "owner_issue": OWNER_ISSUES["cli_parity"],
                "baseline_status": "existing",
                "new_or_existing": "existing",
            })
            continue
        m = re.match(r"  FAIL: (.+?) \(exit=", line)
        if m:
            receipt["items"].append({
                "check_id": "cli_parity",
                "item_id": m.group(1).strip(),
                "result": "fail",
                "category": "cli-parity",
                "owner_issue": OWNER_ISSUES["cli_parity"],
                "baseline_status": "existing",
                "new_or_existing": "existing",
            })
            continue

    # Diagnostic parity failures
    for line in clean_lines:
        m = re.match(r"  FAIL: (\S+) \(selfhost:", line)
        if m:
            receipt["items"].append({
                "check_id": "diag_parity",
                "item_id": m.group(1),
                "result": "fail",
                "category": "diag-parity",
                "owner_issue": OWNER_ISSUES["diag_parity"],
                "baseline_status": "existing",
                "new_or_existing": "existing",
            })

    # Fixpoint hashes
    for line in clean_lines:
        m = re.search(r"sha256\(s2\) = ([a-f0-9]+)", line)
        if m:
            receipt["items"].append({
                "check_id": "fixpoint",
                "item_id": "s2_hash",
                "result": "fail",
                "category": "bootstrap",
                "owner_issue": OWNER_ISSUES["fixpoint"],
                "baseline_status": "existing",
                "new_or_existing": "existing",
                "detail": m.group(1),
            })
            continue
        m = re.search(r"sha256\(s3\) = ([a-f0-9]+)", line)
        if m:
            receipt["items"].append({
                "check_id": "fixpoint",
                "item_id": "s3_hash",
                "result": "fail",
                "category": "bootstrap",
                "owner_issue": OWNER_ISSUES["fixpoint"],
                "baseline_status": "existing",
                "new_or_existing": "existing",
                "detail": m.group(1),
            })
            continue

    # ── Deduplicate aggregate checks ────────────────────────────────
    # When combining outputs from multiple verify subcommands, the same
    # check_id may appear multiple times (e.g. quick_checks from each
    # subcommand's Summary block). Keep only the first occurrence of each
    # check_id, but preserve the one with the most detail (highest
    # pass+fail+skip count).
    seen: dict[str, dict] = {}
    for check in receipt["aggregate_checks"]:
        cid = check["check_id"]
        if cid not in seen:
            seen[cid] = check
        else:
            # Keep the one with higher total count
            existing_total = seen[cid]["pass_count"] + seen[cid]["fail_count"] + seen[cid]["skip_count"]
            new_total = check["pass_count"] + check["fail_count"] + check["skip_count"]
            if new_total > existing_total:
                seen[cid] = check
    receipt["aggregate_checks"] = list(seen.values())

    # ── Deduplicate items ───────────────────────────────────────────
    # Keep only unique (check_id, item_id) pairs
    seen_items: set[tuple[str, str]] = set()
    unique_items = []
    for item in receipt["items"]:
        key = (item["check_id"], item["item_id"])
        if key not in seen_items:
            seen_items.add(key)
            unique_items.append(item)
    receipt["items"] = unique_items

    # ── Summary ─────────────────────────────────────────────────────
    total_items = len(receipt["items"])
    fail_items = sum(1 for i in receipt["items"] if i["result"] == "fail")
    skip_items = sum(1 for i in receipt["items"] if i["result"] == "skip")
    receipt["summary"] = {
        "total_aggregate_checks": len(receipt["aggregate_checks"]),
        "aggregate_failures": sum(1 for c in receipt["aggregate_checks"] if c["result"] == "fail"),
        "total_items": total_items,
        "item_failures": fail_items,
        "item_skips": skip_items,
    }

    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description="Write verify-full receipt")
    parser.add_argument("--input", help="Input file (default: stdin)")
    parser.add_argument("--output", default="docs/data/verify-full-receipt.json",
                        help="Output JSON path")
    parser.add_argument("--exit-status", type=int, default=None,
                        help="Exit status of verify full command")
    args = parser.parse_args()

    if args.input:
        text = Path(args.input).read_text(encoding="utf-8", errors="replace")
    else:
        text = sys.stdin.read()

    receipt = parse_receipt(text)
    if args.exit_status is not None:
        receipt["exit_status"] = args.exit_status

    out = Path(args.output)
    if not out.is_absolute():
        out = REPO_ROOT / out
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    print(f"Receipt written to {out}")
    print(f"  Aggregate checks: {receipt['summary']['total_aggregate_checks']}")
    print(f"  Aggregate failures: {receipt['summary']['aggregate_failures']}")
    print(f"  Individual items: {receipt['summary']['total_items']}")
    print(f"  Item failures: {receipt['summary']['item_failures']}")
    print(f"  Item skips: {receipt['summary']['item_skips']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
