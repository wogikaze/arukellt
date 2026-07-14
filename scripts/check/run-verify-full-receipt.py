#!/usr/bin/env python3
"""Convert `python3 scripts/manager.py verify full` output to a JSON receipt.

Usage:
    python3 scripts/manager.py verify full 2>&1 | \
        python3 scripts/check/run-verify-full-receipt.py > docs/data/verify-full-receipt.json
    python3 scripts/check/run-verify-full-receipt.py --input /tmp/verify_full.log \
        --output docs/data/verify-full-receipt.json
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
RECEIPT_VERSION = "2.0.0"

ANSI_RE = re.compile(r"\x1b\[[0-9;]*m")
CHECK_RE = re.compile(r"^(\u2713|\u2717|\u2299)\s+(.+)$")  # ✓ ✗ ⊙
SECTION_RE = re.compile(r"^\[(\w+)\].*")
SUMMARY_RE = re.compile(r"^(Total checks|Passed|Skipped|Failed):\s*(.+)$")
FIXTURE_PARITY_RE = re.compile(
    r"^(fixture-parity|diag-parity|fmt-parity|cli-parity):\s*PASS=(\d+)\s+FAIL=(\d+)\s+SKIP=(\d+)"
)
FIXTURE_PARITY_RE_NARROW = re.compile(
    r"^(fixture-parity):\s*PASS=(\d+)\s+FAIL=(\d+)\s+SKIP=(\d+)\s+\(wasm-invalid=(\d+)\)"
)
T3_SUMMARY_RE = re.compile(
    r"^T3 WASM validation:\s*(\d+) pass,\s*(\d+) validate-fail,\s*(\d+) compile-fail,\s*(\d+) skip"
)
ITEM_RE = re.compile(
    r"^\s+(FAIL:|pass:|skip:|note:|VALIDATE FAIL:|COMPILE FAIL:|  FAIL:)", re.IGNORECASE
)


def strip_ansi(text: str) -> str:
    return ANSI_RE.sub("", text)


def git_head() -> str:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=REPO_ROOT, capture_output=True, text=True
    )
    return result.stdout.strip() if result.returncode == 0 else ""


def parse_input(text: str) -> dict:
    lines = [strip_ansi(line) for line in text.splitlines()]
    checks: list[dict] = []
    current: dict | None = None
    fixture_totals = {"pass": 0, "fail": 0, "skip": 0, "wasm_invalid": 0}
    t3_totals = {"pass": 0, "validate_fail": 0, "compile_fail": 0, "skip": 0}
    overall_total = overall_passed = overall_skipped = overall_failed = 0

    i = 0
    while i < len(lines):
        line = lines[i]

        # Top-level check pass/fail/skip
        m = CHECK_RE.match(line)
        if m:
            symbol, label = m.group(1), m.group(2).strip()
            # Summary echoes and aggregate parity footers are not checks
            if any(
                label.startswith(x)
                for x in [
                    "Some harness checks",
                    "All selected harness checks",
                    "fixture parity:",
                    "diag parity:",
                    "fmt parity:",
                    "cli parity:",
                ]
            ):
                i += 1
                continue
            status = {"\u2713": "pass", "\u2717": "fail", "\u2299": "skip"}[symbol]
            current = {
                "id": label,
                "status": status,
                "category": "",
                "command": "",
                "primary_path": "",
                "message": "",
                "items": [],
            }
            checks.append(current)
            i += 1
            # Collect optional metadata lines
            while i < len(lines):
                meta = lines[i]
                if meta.startswith("  category:"):
                    current["category"] = meta.split(":", 1)[1].strip()
                    i += 1
                elif meta.startswith("  command:"):
                    current["command"] = meta.split(":", 1)[1].strip()
                    i += 1
                elif meta.startswith("  primary path:"):
                    current["primary_path"] = meta.split(":", 1)[1].strip()
                    i += 1
                else:
                    break
            continue

        # Aggregate parity summaries embedded in the output
        mp = FIXTURE_PARITY_RE_NARROW.match(line)
        if mp:
            name, p, f, s, w = mp.groups()
            fixture_totals["pass"] += int(p)
            fixture_totals["fail"] += int(f)
            fixture_totals["skip"] += int(s)
            fixture_totals["wasm_invalid"] += int(w)
            # Add a synthetic aggregate check if not already present
            checks.append({
                "id": name,
                "status": "fail" if int(f) > 0 else "pass",
                "category": "fixture",
                "command": "python3 scripts/manager.py selfhost parity --mode --fixture",
                "primary_path": "tests/fixtures/manifest.txt",
                "message": f"PASS={p} FAIL={f} SKIP={s} wasm-invalid={w}",
                "items": [],
            })
            i += 1
            continue

        mp = FIXTURE_PARITY_RE.match(line)
        if mp:
            name, p, f, s = mp.groups()
            fixture_totals["pass"] += int(p)
            fixture_totals["fail"] += int(f)
            fixture_totals["skip"] += int(s)
            checks.append({
                "id": name,
                "status": "fail" if int(f) > 0 else "pass",
                "category": "fixture",
                "command": f"python3 scripts/manager.py selfhost {name.split('-')[0]}-parity",
                "primary_path": "tests/fixtures/manifest.txt",
                "message": f"PASS={p} FAIL={f} SKIP={s}",
                "items": [],
            })
            i += 1
            continue

        mt = T3_SUMMARY_RE.match(line)
        if mt:
            t3_totals["pass"] = int(mt.group(1))
            t3_totals["validate_fail"] = int(mt.group(2))
            t3_totals["compile_fail"] = int(mt.group(3))
            t3_totals["skip"] = int(mt.group(4))
            i += 1
            continue

        ms = SUMMARY_RE.match(line)
        if ms:
            key, val = ms.group(1), ms.group(2).strip()
            # Strip ANSI/colors already stripped, but the value may have trailing words
            num = re.search(r"\d+", val)
            n = int(num.group()) if num else 0
            if key == "Total checks":
                overall_total = n
            elif key == "Passed":
                overall_passed = n
            elif key == "Skipped":
                overall_skipped = n
            elif key == "Failed":
                overall_failed = n
            i += 1
            continue

        # Item lines belonging to the most recent failing check
        if current is not None and current["status"] == "fail":
            if line.startswith("  FAIL:") or line.startswith("  pass:") or line.startswith("  skip:") or line.startswith("  note:"):
                current["items"].append(line.strip())
                i += 1
                # Include continuation detail lines (expected/current/exit) indented further
                while i < len(lines) and (lines[i].startswith("    expected:") or lines[i].startswith("    current :") or lines[i].startswith("    exit:") or lines[i].startswith("    pinned :") or lines[i].startswith("    current:")):
                    current["items"].append(lines[i].strip())
                    i += 1
                continue
            if line.startswith("VALIDATE FAIL:") or line.startswith("COMPILE FAIL:") or line.startswith("COMPILE TIMEOUT:"):
                current["items"].append(line.strip())
                i += 1
                continue
            # Generic detail continuation lines (e.g. compiler-boundary violations)
            if line.startswith("  ") and not line.startswith("   ") and not line.startswith("  category:") and not line.startswith("  command:") and not line.startswith("  primary path:"):
                current["items"].append(line.strip())
                i += 1
                continue

        i += 1

    # If no top-level checks parsed, derive summary from aggregate lines
    if not checks and (fixture_totals["fail"] or t3_totals["validate_fail"] or t3_totals["compile_fail"]):
        overall_failed = 1
    else:
        overall_total = max(overall_total, len(checks))
        overall_passed = sum(1 for c in checks if c["status"] == "pass")
        overall_skipped = sum(1 for c in checks if c["status"] == "skip")
        overall_failed = sum(1 for c in checks if c["status"] == "fail")

    status = "pass" if overall_failed == 0 else "fail"

    return {
        "receipt_version": RECEIPT_VERSION,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "verified_commit": git_head(),
        "verification_command": "python3 scripts/manager.py verify full",
        "status": status,
        "summary": {
            "checks_total": overall_total,
            "checks_passed": overall_passed,
            "checks_failed": overall_failed,
            "checks_skipped": overall_skipped,
            "background_pass": overall_passed,
            "background_fail": overall_failed,
            "fixture_pass": fixture_totals["pass"],
            "fixture_fail": fixture_totals["fail"],
            "fixture_skip": fixture_totals["skip"],
            "wasm_invalid": fixture_totals["wasm_invalid"],
            "t3_pass": t3_totals["pass"],
            "t3_validate_fail": t3_totals["validate_fail"],
            "t3_compile_fail": t3_totals["compile_fail"],
            "t3_skip": t3_totals["skip"],
            "incidents": [],
        },
        "checks": checks,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate verify full JSON receipt")
    parser.add_argument("--input", help="Path to verify full text output (default: stdin)")
    parser.add_argument("--output", help="Write receipt to this path")
    parser.add_argument("--exit-status", type=int, help="Override overall status from verify exit code")
    args = parser.parse_args()

    if args.input:
        text = Path(args.input).read_text(encoding="utf-8")
    else:
        text = sys.stdin.read()

    receipt = parse_input(text)
    if args.exit_status is not None:
        receipt["status"] = "pass" if args.exit_status == 0 else "fail"

    out = json.dumps(receipt, indent=2, ensure_ascii=False)
    if args.output:
        Path(args.output).write_text(out + "\n", encoding="utf-8")
        print(f"wrote receipt to {args.output}")
    else:
        print(out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
