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

        # T3 WASM validation aggregate summary (emitted at the end of output)
        m = re.search(r"T3 WASM validation:\s*(\d+) pass,\s*(\d+) validate-fail,\s*(\d+) compile-fail,\s*(\d+) skip", line)
        if m:
            pass_count = int(m.group(1))
            validate_fail = int(m.group(2))
            compile_fail = int(m.group(3))
            skip_count = int(m.group(4))
            # Update the most recent t3_wasm_validate aggregate if present
            for agg in reversed(receipt["aggregate_checks"]):
                if agg.get("check_id") == "t3_wasm_validate":
                    agg["pass_count"] = pass_count
                    agg["fail_count"] = validate_fail + compile_fail
                    agg["skip_count"] = skip_count
                    agg["result"] = "fail" if (validate_fail + compile_fail) > 0 else "pass"
                    break
            else:
                receipt["aggregate_checks"].append({
                    "check_id": "t3_wasm_validate",
                    "category": "wasm-validation",
                    "result": "fail" if (validate_fail + compile_fail) > 0 else "pass",
                    "pass_count": pass_count,
                    "fail_count": validate_fail + compile_fail,
                    "skip_count": skip_count,
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
            item_id = m.group(1)
            # Distinguish fixture parity failures from CLI/diagnostic parity failures
            if not item_id.endswith(".ark"):
                continue
            if " (selfhost:" in line or " (no-args:" in line or "drifts from golden" in line or "(exit=" in line:
                continue
            receipt["items"].append({
                "check_id": "fixture_parity",
                "item_id": item_id,
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

    # T3 WASM validation failures / compile failures / timeouts
    t3_item_re = re.compile(r"^\s*(VALIDATE FAIL|COMPILE FAIL|COMPILE TIMEOUT): (\S+)(.*)")
    for line in clean_lines:
        m = t3_item_re.match(line)
        if m:
            kind = m.group(1)
            fixture = m.group(2)
            if kind == "VALIDATE FAIL" or kind == "COMPILE FAIL":
                result = "fail"
                owner = OWNER_ISSUES["t3_wasm_validate"]
            else:
                result = "skip"
                owner = OWNER_ISSUES["skip_debt"]
            receipt["items"].append({
                "check_id": "t3_wasm_validate",
                "item_id": fixture,
                "result": result,
                "category": "wasm-validation",
                "owner_issue": owner,
                "baseline_status": "existing",
                "new_or_existing": "existing",
                "detail": m.group(3).strip(),
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


def _normalize_receipt(receipt: dict) -> dict:
    """Restructure aggregate_checks and top-level items into checks[].items."""
    # Aggregate checks keyed by check_id; prefer the entry with the most detail
    checks_by_id: dict[str, dict] = {}
    for agg in receipt.get("aggregate_checks", []):
        cid = agg.get("check_id", "unknown")
        existing = checks_by_id.get(cid)
        if existing is None:
            checks_by_id[cid] = agg.copy()
        else:
            # Keep the richer aggregate (larger total count)
            existing_total = existing.get("pass_count", 0) + existing.get("fail_count", 0) + existing.get("skip_count", 0)
            new_total = agg.get("pass_count", 0) + agg.get("fail_count", 0) + agg.get("skip_count", 0)
            if new_total > existing_total:
                checks_by_id[cid] = agg.copy()

    # Build checks with nested items
    checks: list[dict] = []
    unmatched_items: list[dict] = []
    for cid in checks_by_id:
        check = checks_by_id[cid]
        # Rename result -> status at the check level
        status = check.pop("result", "fail")
        check["status"] = status
        check["items"] = []
        checks.append(check)

    # Attach items to their owning check
    for item in receipt.get("items", []):
        cid = item.get("check_id", "unknown")
        if cid in checks_by_id:
            checks_by_id[cid]["items"].append(item)
        else:
            unmatched_items.append(item)

    # Any items without a matching check become a synthetic check
    if unmatched_items:
        for item in unmatched_items:
            cid = item.get("check_id", "unknown")
            if cid not in checks_by_id:
                checks_by_id[cid] = {
                    "check_id": cid,
                    "status": "fail",
                    "category": "unknown",
                    "owner_issue": None,
                    "items": [],
                }
                checks.append(checks_by_id[cid])
            checks_by_id[cid]["items"].append(item)

    # Sort checks deterministically
    checks.sort(key=lambda c: c.get("check_id", ""))

    # Add check-level command/primary_path helpers where missing
    command_paths = {
        "fixture_parity": "python3 scripts/manager.py selfhost fixture-parity",
        "diag_parity": "python3 scripts/manager.py selfhost diag-parity",
        "cli_parity": "python3 scripts/manager.py selfhost parity --mode --cli",
        "fmt_parity": "python3 scripts/manager.py selfhost fmt-parity",
        "wat_roundtrip": "bash scripts/run/wat-roundtrip.sh",
        "component_interop": "python3 scripts/manager.py verify component",
        "t3_wasm_validate": "python3 scripts/check/check-t3-wasm-validate.py",
        "fixpoint": "python3 scripts/manager.py verify --selfhost-fixpoint",
        "quick_checks": "python3 scripts/manager.py verify quick",
    }
    primary_paths = {
        "fixture_parity": "tests/fixtures/manifest.txt",
        "diag_parity": "tests/fixtures/",
        "cli_parity": "tests/snapshots/selfhost/",
        "fmt_parity": "tests/fixtures/",
        "wat_roundtrip": "scripts/run/wat-roundtrip.sh",
        "component_interop": "tests/component-interop/",
        "t3_wasm_validate": "scripts/check/check-t3-wasm-validate.py",
        "fixpoint": "scripts/selfhost/checks.py",
        "quick_checks": "scripts/manager.py",
    }
    for check in checks:
        cid = check.get("check_id", "")
        if not check.get("command") and cid in command_paths:
            check["command"] = command_paths[cid]
        if not check.get("primary_path") and cid in primary_paths:
            check["primary_path"] = primary_paths[cid]

    # Compute summary
    total_checks = len(checks)
    passed_checks = sum(1 for c in checks if c.get("status") == "pass")
    failed_checks = sum(1 for c in checks if c.get("status") == "fail")
    skipped_checks = sum(1 for c in checks if c.get("status") == "skip")
    total_items = sum(len(c.get("items", [])) for c in checks)
    fail_items = sum(
        1 for c in checks for i in c.get("items", []) if i.get("result") == "fail"
    )
    skip_items = sum(
        1 for c in checks for i in c.get("items", []) if i.get("result") == "skip"
    )

    # Per-domain totals
    t3_check = checks_by_id.get("t3_wasm_validate", {})
    fixture_check = checks_by_id.get("fixture_parity", {})
    summary = {
        "checks_total": total_checks,
        "checks_passed": passed_checks,
        "checks_failed": failed_checks,
        "checks_skipped": skipped_checks,
        "total_items": total_items,
        "item_failures": fail_items,
        "item_skips": skip_items,
        "t3_pass": t3_check.get("pass_count", 0),
        "t3_validate_fail": t3_check.get("fail_count", 0) - 0,  # kept as fail count
        "t3_compile_fail": 0,
        "t3_skip": t3_check.get("skip_count", 0),
        "fixture_pass": fixture_check.get("pass_count", 0),
        "fixture_fail": fixture_check.get("fail_count", 0),
        "fixture_skip": fixture_check.get("skip_count", 0),
        "incidents": [],
    }

    receipt["receipt_version"] = "2.0.0"
    receipt["schema_version"] = 2
    receipt["status"] = "pass" if failed_checks == 0 else "fail"
    receipt["checks"] = checks
    receipt["summary"] = summary
    # Remove legacy keys
    receipt.pop("aggregate_checks", None)
    receipt.pop("items", None)
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

    _normalize_receipt(receipt)

    out = Path(args.output)
    if not out.is_absolute():
        out = REPO_ROOT / out
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    print(f"Receipt written to {out}")
    print(f"  Checks: {receipt['summary']['checks_total']} "
          f"(passed={receipt['summary']['checks_passed']} "
          f"failed={receipt['summary']['checks_failed']} "
          f"skipped={receipt['summary']['checks_skipped']})")
    print(f"  Items: {receipt['summary']['total_items']} "
          f"(failures={receipt['summary']['item_failures']} "
          f"skips={receipt['summary']['item_skips']})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
