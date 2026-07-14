#!/usr/bin/env python3
"""Write a machine-readable verify-full receipt from captured output.

This script parses the stdout of `python3 scripts/manager.py verify full` and
writes a structured JSON receipt with per-check and per-item identity.

The receipt schema separates aggregate checks from individual items:
- checks: summary-level pass/fail/skip counts per domain, plus identity coverage
- items: individual fixture/test IDs with result, category, owner_issue

Usage:
    python3 scripts/manager.py verify full 2>&1 | \
    python3 scripts/gen/write-verify-receipt.py --output docs/data/verify-full-receipt.json

Or via captured file:
    python3 scripts/gen/write-verify-receipt.py --input verify_full.log --output docs/data/verify-full-receipt.json

The verify runner should record the start timestamp and pass it in:
    START=$(date -Iseconds)
    python3 scripts/manager.py verify full 2>&1 | \
    python3 scripts/gen/write-verify-receipt.py --started-at "$START" --output docs/data/verify-full-receipt.json
"""

from __future__ import annotations

import argparse
import datetime
import json
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
ANSI_RE = re.compile(r"\x1b\[[0-9;]*m")

# Canonical issue owners for each verify domain. The quick_checks harness fails
# when the T3 background gate fails, so its owner is the same as the T3 gate.
OWNER_ISSUES = {
    "size": "422",
    "quick_checks": "808",
    "t3_wasm_validate": "808",
    "fixture_parity": "807",
    "wat_roundtrip": "809",
    "component_interop": "810",
    "cli_parity": "811",
    "diag_parity": "812",
    "fixpoint": "813",
}

# How completely this domain emits per-item identity in the log.
# - full: every aggregate result has a stable item ID in the receipt.
# - partial: only some items are emitted (domain does not print every identity).
# - aggregate_only: no per-item identity expected (size, quick, fixpoint).
IDENTITY_COVERAGE = {
    "size": "aggregate_only",
    "quick_checks": "aggregate_only",
    "t3_wasm_validate": "full",
    "fixture_parity": "full",
    "wat_roundtrip": "full",
    "component_interop": "full",
    "cli_parity": "full",
    "diag_parity": "full",
    "fixpoint": "aggregate_only",
}

# Result of a single aggregate check.
RESULT_PASS = "pass"
RESULT_FAIL = "fail"
RESULT_SKIP = "skip"


def _clean(text: str) -> str:
    """Strip ANSI colour codes."""
    return ANSI_RE.sub("", text)


def _short_commit(full: str) -> str:
    """Return an 8-character short commit hash if the input looks like one."""
    if re.fullmatch(r"[0-9a-f]{40}", full):
        return full[:8]
    return full


def _get_git_commit() -> str:
    """Return the current HEAD commit (short)."""
    try:
        return _short_commit(
            subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=REPO_ROOT, text=True
            ).strip()
        )
    except (subprocess.CalledProcessError, FileNotFoundError):
        return "unknown"


def _parse_iso_timestamp(value: str | None) -> str | None:
    """Validate an ISO 8601 timestamp. Returns the value or None if invalid."""
    if not value:
        return None
    # Python 3.11+ supports datetime.datetime.fromisoformat with 'Z'.
    try:
        dt = datetime.datetime.fromisoformat(value.replace("Z", "+00:00"))
        return dt.isoformat().replace("+00:00", "Z")
    except ValueError:
        return None


class _Check:
    """Aggregate check result."""

    def __init__(
        self,
        check_id: str,
        pass_count: int = 0,
        fail_count: int = 0,
        skip_count: int = 0,
        result: str = RESULT_PASS,
        owner_issue: str | None = None,
        identity_coverage: str = "aggregate_only",
        command: str | None = None,
        primary_path: str | None = None,
    ):
        self.check_id = check_id
        self.pass_count = pass_count
        self.fail_count = fail_count
        self.skip_count = skip_count
        self.result = result
        self.owner_issue = owner_issue
        self.identity_coverage = identity_coverage
        self.command = command
        self.primary_path = primary_path
        self.items: list[dict] = []

    def as_dict(self) -> dict:
        return {
            "check_id": self.check_id,
            "pass_count": self.pass_count,
            "fail_count": self.fail_count,
            "skip_count": self.skip_count,
            "result": self.result,
            "owner_issue": self.owner_issue,
            "identity_coverage": self.identity_coverage,
            "command": self.command,
            "primary_path": self.primary_path,
            "items": self.items,
        }


class _Section:
    """A contiguous block of the log belonging to a single domain."""

    def __init__(self, check_id: str, start_idx: int, end_idx: int | None = None):
        self.check_id = check_id
        self.start_idx = start_idx
        self.end_idx = end_idx


class ReceiptWriter:
    """Parse `verify full` output and emit a domain-aware receipt."""

    # Each section is identified by a header line. The end is the first line
    # matching the end pattern or the start of the next section.
    SECTION_STARTERS = [
        ("size", re.compile(r"^\[size\] Checking hello\.wasm binary size gate")),
        ("quick", re.compile(r"^\[bg\] Running background checks in parallel")),
        ("fixtures", re.compile(r"^\[fixtures\] Running selfhost fixture parity")),
        ("wat", re.compile(r"^\[wat\] Running WAT roundtrip gate")),
        ("component", re.compile(r"^\[component\] Component interop smoke test")),
        ("cli_parity", re.compile(r"^\[cli-parity\] Checking selfhost CLI surface")),
        ("diag_parity", re.compile(r"^\[diag-parity\] Checking .* diag: fixtures" )),
        ("fixpoint", re.compile(r"^\[selfhost-fixpoint\] Fixpoint gate \(full verify\)")),
    ]

    # Regexes for aggregate summaries.
    T3_SUMMARY_RE = re.compile(
        r"^T3 WASM validation:\s+(\d+) pass,\s+(\d+) validate-fail,\s+(\d+) compile-fail,\s+(\d+) skip"
    )
    FIXTURE_PARITY_RE = re.compile(
        r"^fixture-parity:\s+PASS=(\d+)\s+FAIL=(\d+)\s+SKIP=(\d+)"
    )
    CLI_PARITY_RE = re.compile(
        r"^cli-parity:\s+PASS=(\d+)\s+FAIL=(\d+)"
    )
    DIAG_PARITY_RE = re.compile(
        r"^diag-parity:\s+PASS=(\d+)\s+SKIP=(\d+)\s+FAIL=(\d+)"
    )
    WAT_SUMMARY_RE = re.compile(
        r"^WAT roundtrip summary:\s+PASS=(\d+)\s+FAIL=(\d+)\s+SKIP=(\d+)"
    )
    TOTAL_CHECKS_RE = re.compile(
        r"^Total checks:\s+(\d+)"
    )
    PASSED_RE = re.compile(r"^Passed:\s+(\d+)")
    SKIPPED_RE = re.compile(r"^Skipped:\s+(\d+)")
    FAILED_RE = re.compile(r"^Failed:\s+(\d+)")

    # Item patterns.
    FAIL_ITEM_RE = re.compile(r"^\s*FAIL:\s+([^()]+?)(?:\s+(?:\(|$))")
    SKIP_ITEM_RE = re.compile(r"^\s*skip:\s+(\S+)")
    WAT_FAIL_RE = re.compile(r"^\s*FAIL:\s+(.+?)\s+\([^)]+\)\s+\(")
    WAT_FAIL_LABEL_RE = re.compile(r"^\s*FAIL:\s+(.+?)$")
    COMPONENT_FAIL_RE = re.compile(r"✗\s+component\s+interop:\s+(\S+)")
    SIZE_PASS_RE = re.compile(r"✓\s+hello\.wasm\s+binary\s+size:\s+(\d+)\s+bytes")
    SIZE_FAIL_RE = re.compile(r"✗\s+hello\.wasm\s+binary\s+size:")
    FIXPOINT_S2_RE = re.compile(r"sha256\(s2\)\s*=\s*([0-9a-f]+)")
    FIXPOINT_S3_RE = re.compile(r"sha256\(s3\)\s*=\s*([0-9a-f]+)")

    def __init__(self, log_text: str, started_at: str | None, exit_status: int):
        self.lines = [self._clean(l) for l in log_text.splitlines()]
        self.started_at = started_at
        self.exit_status = exit_status
        self.sections = self._detect_sections()
        self.checks: dict[str, _Check] = {}
        self.incidents: list[str] = []

    def _clean(self, line: str) -> str:
        return ANSI_RE.sub("", line).rstrip()

    def _detect_sections(self) -> list[_Section]:
        """Detect contiguous sections in the log."""
        sections: list[_Section] = []
        n = len(self.lines)
        for i, line in enumerate(self.lines):
            for check_id, pattern in self.SECTION_STARTERS:
                if pattern.search(line):
                    sections.append(_Section(check_id, i))
                    break
        # Sort and resolve end indices to the next section start.
        sections.sort(key=lambda s: s.start_idx)
        for idx, sec in enumerate(sections):
            if idx + 1 < len(sections):
                sec.end_idx = sections[idx + 1].start_idx
            else:
                sec.end_idx = n
        return sections

    def _section_lines(self, check_id: str) -> list[str]:
        for sec in self.sections:
            if sec.check_id == check_id:
                return self.lines[sec.start_idx : sec.end_idx]
        return []

    def _make_check(self, check_id: str) -> _Check:
        if check_id not in self.checks:
            self.checks[check_id] = _Check(
                check_id=check_id,
                owner_issue=OWNER_ISSUES.get(check_id),
                identity_coverage=IDENTITY_COVERAGE.get(check_id, "aggregate_only"),
            )
        return self.checks[check_id]

    def _add_item(
        self,
        check_id: str,
        item_id: str,
        result: str,
        category: str,
        detail: str | None = None,
        baseline_status: str = "existing",
        new_or_existing: str = "existing",
    ) -> None:
        check = self._make_check(check_id)
        item = {
            "check_id": check_id,
            "item_id": item_id,
            "result": result,
            "category": category,
            "owner_issue": check.owner_issue,
            "baseline_status": baseline_status,
            "new_or_existing": new_or_existing,
        }
        if detail:
            item["detail"] = detail
        check.items.append(item)

    def _parse_size(self) -> None:
        check = self._make_check("size")
        check.identity_coverage = "aggregate_only"
        check.command = "python3 scripts/manager.py verify size"
        check.primary_path = "src/compiler/emitter.ark"
        section = self._section_lines("size")
        for line in section:
            m = self.SIZE_PASS_RE.search(line)
            if m:
                check.pass_count = 1
                check.result = RESULT_PASS
                check.items.append({
                    "check_id": "size",
                    "item_id": "hello.wasm binary size",
                    "result": RESULT_PASS,
                    "category": "size",
                    "owner_issue": check.owner_issue,
                    "baseline_status": "existing",
                    "new_or_existing": "existing",
                    "detail": f"{m.group(1)} bytes <= 5120",
                })
                return
            if self.SIZE_FAIL_RE.search(line):
                check.pass_count = 0
                check.fail_count = 1
                check.result = RESULT_FAIL
                check.items.append({
                    "check_id": "size",
                    "item_id": "hello.wasm binary size",
                    "result": RESULT_FAIL,
                    "category": "size",
                    "owner_issue": check.owner_issue,
                    "baseline_status": "existing",
                    "new_or_existing": "existing",
                })
                return
        # Section not found or not run; mark as skip.
        check.skip_count = 1
        check.result = RESULT_SKIP

    def _parse_quick(self) -> None:
        section = self._section_lines("quick")
        t3_pass = t3_fail = 0
        t3_aggregate_seen = False
        quick_pass = quick_fail = quick_skip = 0
        for line in section:
            m = self.T3_SUMMARY_RE.search(line)
            if m:
                t3_pass = int(m.group(1))
                t3_fail = int(m.group(2))
                t3_aggregate_seen = True
        for idx, line in enumerate(section):
            m = self.TOTAL_CHECKS_RE.search(line)
            if m:
                quick_pass = quick_fail = quick_skip = 0
                total = int(m.group(1))
                # The next three lines are Passed, Skipped, Failed.
                for nxt in section[idx + 1 : idx + 5]:
                    pm = self.PASSED_RE.search(nxt)
                    if pm:
                        quick_pass = int(pm.group(1))
                    sm = self.SKIPPED_RE.search(nxt)
                    if sm:
                        quick_skip = int(sm.group(1))
                    fm = self.FAILED_RE.search(nxt)
                    if fm:
                        quick_fail = int(fm.group(1))
        quick_check = self._make_check("quick_checks")
        quick_check.command = "python3 scripts/manager.py verify quick"
        quick_check.primary_path = "scripts/manager.py"
        quick_check.pass_count = quick_pass
        quick_check.fail_count = quick_fail
        quick_check.skip_count = quick_skip
        quick_check.result = RESULT_FAIL if quick_fail else RESULT_PASS

        # T3 aggregate domain is populated from the report file, but we can also
        # derive it from the log summary if the report is missing.
        if t3_aggregate_seen:
            self._parse_t3_report(t3_pass, t3_fail)
        else:
            self._parse_t3_report(0, 0)

    def _parse_t3_report(self, pass_count: int, fail_count: int) -> None:
        check = self._make_check("t3_wasm_validate")
        check.command = "python3 scripts/check/check-t3-wasm-validate.py"
        check.primary_path = "scripts/check/check-t3-wasm-validate.py"
        report_path = REPO_ROOT / ".build" / "t3-wasm-validate-report.json"
        if report_path.is_file():
            try:
                report = json.loads(report_path.read_text(encoding="utf-8"))
            except (json.JSONDecodeError, OSError):
                report = None
        else:
            report = None

        if report is not None:
            check.pass_count = report.get("pass_count", pass_count)
            check.fail_count = report.get("fail_validate", fail_count)
            check.skip_count = report.get("skip_count", 0)
            for item in report.get("items", []):
                status = item.get("status", "")
                fixture = item.get("fixture", "")
                detail = item.get("detail", "")
                if status == "skip":
                    result = RESULT_SKIP
                    category = "t3-compile-skip"
                elif status == "compile-fail":
                    result = RESULT_FAIL
                    category = "t3-compile-failure"
                elif status == "timeout":
                    result = RESULT_SKIP
                    category = "t3-compile-timeout"
                else:
                    result = RESULT_FAIL
                    category = "t3-wasm-validate-failure"
                self._add_item(
                    "t3_wasm_validate",
                    fixture,
                    result,
                    category,
                    detail,
                    baseline_status="existing",
                    new_or_existing="existing",
                )
        else:
            # Fallback: log-based parsing only (truncated by manager.py).
            check.pass_count = pass_count
            check.fail_count = fail_count
            check.skip_count = 0
            section = self._section_lines("quick")
            for line in section:
                if line.startswith("VALIDATE FAIL:"):
                    fixture = line.split("—", 1)[0].replace("VALIDATE FAIL:", "").strip()
                    detail = line.split("—", 1)[1].strip() if "—" in line else ""
                    self._add_item("t3_wasm_validate", fixture, RESULT_FAIL, "t3-wasm-validate-failure", detail)
                elif line.startswith("COMPILE FAIL:"):
                    fixture = line.replace("COMPILE FAIL:", "").strip()
                    self._add_item("t3_wasm_validate", fixture, RESULT_FAIL, "t3-compile-failure", "compile failed")
                elif line.startswith("COMPILE TIMEOUT:"):
                    fixture = line.replace("COMPILE TIMEOUT:", "").strip()
                    self._add_item("t3_wasm_validate", fixture, RESULT_SKIP, "t3-compile-timeout", "compile timeout")

        check.result = RESULT_FAIL if check.fail_count else RESULT_PASS

    def _parse_fixtures(self) -> None:
        section = self._section_lines("fixtures")
        check = self._make_check("fixture_parity")
        check.command = "python3 scripts/manager.py verify fixtures"
        check.primary_path = "tests/fixtures/manifest.txt"

        for line in section:
            m = self.FIXTURE_PARITY_RE.search(line)
            if m:
                check.pass_count = int(m.group(1))
                check.fail_count = int(m.group(2))
                check.skip_count = int(m.group(3))
                check.result = RESULT_FAIL if check.fail_count else RESULT_PASS
                break

        for line in section:
            if line.startswith("  FAIL:"):
                m = self.FAIL_ITEM_RE.match(line)
                if m:
                    fixture = m.group(1).strip()
                    if fixture.endswith(".ark"):
                        detail = line.split("(", 1)[1].rstrip(")") if "(" in line else ""
                        self._add_item("fixture_parity", fixture, RESULT_FAIL, "fixture-parity", detail)
            elif line.startswith("  skip:"):
                m = self.SKIP_ITEM_RE.match(line)
                if m:
                    fixture = m.group(1).strip()
                    if fixture.endswith(".ark"):
                        detail = line.split("(", 1)[1].rstrip(")") if "(" in line else ""
                        self._add_item("fixture_parity", fixture, RESULT_SKIP, "fixture-parity", detail)

    def _parse_wat(self) -> None:
        section = self._section_lines("wat")
        check = self._make_check("wat_roundtrip")
        check.command = "bash scripts/run/wat-roundtrip.sh"
        check.primary_path = "scripts/run/wat-roundtrip.sh"

        for line in section:
            m = self.WAT_SUMMARY_RE.search(line)
            if m:
                check.pass_count = int(m.group(1))
                check.fail_count = int(m.group(2))
                check.skip_count = int(m.group(3))
                check.result = RESULT_FAIL if check.fail_count else RESULT_PASS
                break

        for line in section:
            if line.startswith("  FAIL:"):
                # WAT failures look like: FAIL: fixture (wasm32) (reason)
                m = self.WAT_FAIL_RE.match(line)
                if m:
                    label = m.group(1).strip()
                    detail = line.split("(", 1)[1].rstrip(")") if "(" in line else ""
                    self._add_item("wat_roundtrip", label, RESULT_FAIL, "wat-roundtrip", detail)
                else:
                    m = self.WAT_FAIL_LABEL_RE.match(line)
                    if m:
                        label = m.group(1).strip()
                        self._add_item("wat_roundtrip", label, RESULT_FAIL, "wat-roundtrip")

    def _parse_component(self) -> None:
        section = self._section_lines("component")
        check = self._make_check("component_interop")
        check.command = "python3 scripts/manager.py verify component"
        check.primary_path = "tests/component-interop/"

        pass_count = fail_count = skip_count = 0
        for line in section:
            if "✓ component interop:" in line:
                pass_count += 1
            elif "✗ component interop:" in line:
                fail_count += 1
                m = self.COMPONENT_FAIL_RE.search(line)
                if m:
                    fixture = m.group(1)
                    self._add_item("component_interop", fixture, RESULT_FAIL, "component-interop")
            elif "⊙ component interop" in line or "skipped" in line.lower():
                # Any skip line is not emitted per-fixture by the Harness.
                pass
        check.pass_count = pass_count
        check.fail_count = fail_count
        check.skip_count = skip_count
        check.result = RESULT_FAIL if fail_count else RESULT_PASS

    def _parse_cli_parity(self) -> None:
        section = self._section_lines("cli_parity")
        check = self._make_check("cli_parity")
        check.command = "python3 scripts/manager.py selfhost parity --mode --cli"
        check.primary_path = "tests/snapshots/selfhost/"

        for line in section:
            m = self.CLI_PARITY_RE.search(line)
            if m:
                check.pass_count = int(m.group(1))
                check.fail_count = int(m.group(2))
                check.skip_count = 0
                check.result = RESULT_FAIL if check.fail_count else RESULT_PASS
                break

        for line in section:
            if line.startswith("  FAIL:"):
                m = self.FAIL_ITEM_RE.match(line)
                if m:
                    item = m.group(1).strip()
                    detail = line.split("(", 1)[1].rstrip(")") if "(" in line else ""
                    self._add_item("cli_parity", item, RESULT_FAIL, "cli-parity", detail)

    def _parse_diag_parity(self) -> None:
        section = self._section_lines("diag_parity")
        check = self._make_check("diag_parity")
        check.command = "python3 scripts/manager.py selfhost diag-parity"
        check.primary_path = "tests/fixtures/"

        for line in section:
            m = self.DIAG_PARITY_RE.search(line)
            if m:
                check.pass_count = int(m.group(1))
                check.skip_count = int(m.group(2))
                check.fail_count = int(m.group(3))
                check.result = RESULT_FAIL if check.fail_count else RESULT_PASS
                break

        for line in section:
            if line.startswith("  FAIL:"):
                m = self.FAIL_ITEM_RE.match(line)
                if m:
                    fixture = m.group(1).strip()
                    if fixture.endswith(".ark"):
                        detail = line.split("(", 1)[1].rstrip(")") if "(" in line else ""
                        self._add_item("diag_parity", fixture, RESULT_FAIL, "diag-parity", detail)
            elif line.startswith("  skip:"):
                m = self.SKIP_ITEM_RE.match(line)
                if m:
                    fixture = m.group(1).strip()
                    if fixture.endswith(".ark"):
                        detail = line.split("(", 1)[1].rstrip(")") if "(" in line else ""
                        self._add_item("diag_parity", fixture, RESULT_SKIP, "diag-parity", detail)

    def _parse_fixpoint(self) -> None:
        section = self._section_lines("fixpoint")
        check = self._make_check("fixpoint")
        check.command = "python3 scripts/manager.py selfhost fixpoint"
        check.primary_path = "scripts/run/arukellt-selfhost.sh"

        s2_hash: str | None = None
        s3_hash: str | None = None
        reached = False
        for line in section:
            m = self.FIXPOINT_S2_RE.search(line)
            if m:
                s2_hash = m.group(1)
            m = self.FIXPOINT_S3_RE.search(line)
            if m:
                s3_hash = m.group(1)
            if "selfhost fixpoint reached" in line or "✓ selfhost fixpoint reached" in line:
                reached = True

        if reached:
            check.pass_count = 1
            check.result = RESULT_PASS
        else:
            check.fail_count = 1
            check.result = RESULT_FAIL

        if s2_hash:
            self._add_item("fixpoint", f"s2:{s2_hash}", check.result, "fixpoint", f"sha256(s2) = {s2_hash}")
        if s3_hash:
            self._add_item("fixpoint", f"s3:{s3_hash}", check.result, "fixpoint", f"sha256(s3) = {s3_hash}")

    def _ensure_invariants(self) -> None:
        """Verify identity/aggregate invariants and record incidents."""
        for check in self.checks.values():
            if check.identity_coverage == "full":
                fail_items = sum(1 for i in check.items if i["result"] == RESULT_FAIL)
                skip_items = sum(1 for i in check.items if i["result"] == RESULT_SKIP)
                if fail_items != check.fail_count:
                    self.incidents.append(
                        f"{check.check_id}: failure identity count ({fail_items}) != "
                        f"aggregate fail_count ({check.fail_count})"
                    )
                if skip_items != check.skip_count:
                    self.incidents.append(
                        f"{check.check_id}: skip identity count ({skip_items}) != "
                        f"aggregate skip_count ({check.skip_count})"
                    )
            if check.identity_coverage in ("full", "partial"):
                # pass + fail + skip identities must not exceed the aggregate total.
                total_items = len(check.items)
                total_agg = check.pass_count + check.fail_count + check.skip_count
                if total_items > total_agg:
                    self.incidents.append(
                        f"{check.check_id}: item identity count ({total_items}) > "
                        f"aggregate total ({total_agg})"
                    )

    def build(self) -> dict:
        """Parse the log and produce the receipt dictionary."""
        self._parse_size()
        self._parse_quick()
        self._parse_fixtures()
        self._parse_wat()
        self._parse_component()
        self._parse_cli_parity()
        self._parse_diag_parity()
        self._parse_fixpoint()
        self._ensure_invariants()

        passed = sum(1 for c in self.checks.values() if c.result == RESULT_PASS)
        failed = sum(1 for c in self.checks.values() if c.result == RESULT_FAIL)
        skipped = sum(1 for c in self.checks.values() if c.result == RESULT_SKIP)

        total_items = sum(len(c.items) for c in self.checks.values())
        item_failures = sum(1 for c in self.checks.values() for i in c.items if i["result"] == RESULT_FAIL)
        item_skips = sum(1 for c in self.checks.values() for i in c.items if i["result"] == RESULT_SKIP)

        t3 = self.checks.get("t3_wasm_validate")
        t3_pass = t3.pass_count if t3 else 0
        t3_fail = t3.fail_count if t3 else 0
        t3_skip = t3.skip_count if t3 else 0

        fixture = self.checks.get("fixture_parity")
        fixture_pass = fixture.pass_count if fixture else 0
        fixture_fail = fixture.fail_count if fixture else 0
        fixture_skip = fixture.skip_count if fixture else 0

        receipt = {
            "schema_version": 2,
            "generated_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
            "started_at": self.started_at,
            "finished_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
            "verified_commit": _get_git_commit(),
            "exit_status": self.exit_status,
            "status": RESULT_FAIL if failed or self.exit_status else RESULT_PASS,
            "summary": {
                "checks_total": len(self.checks),
                "checks_passed": passed,
                "checks_failed": failed,
                "checks_skipped": skipped,
                "total_items": total_items,
                "item_failures": item_failures,
                "item_skips": item_skips,
                "t3_pass": t3_pass,
                "t3_fail": t3_fail,
                "t3_skip": t3_skip,
                "fixture_pass": fixture_pass,
                "fixture_fail": fixture_fail,
                "fixture_skip": fixture_skip,
                "incidents": self.incidents,
            },
            "checks": [c.as_dict() for c in self.checks.values()],
        }
        return receipt


def _normalize_receipt(receipt: dict) -> dict:
    """Sort and normalize the receipt for stable output."""
    order = ["schema_version", "generated_at", "started_at", "finished_at", "verified_commit", "exit_status", "status", "summary", "checks"]
    return {k: receipt[k] for k in order if k in receipt}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Write a verify-full receipt from captured output")
    parser.add_argument("--input", "-i", default="-", help="Input log file (default: stdin)")
    parser.add_argument("--output", "-o", required=True, help="Output JSON receipt path")
    parser.add_argument("--started-at", help="ISO 8601 timestamp when verify full started")
    parser.add_argument("--exit-status", type=int, default=0, help="Shell exit status of the verify command")
    args = parser.parse_args(argv)

    if args.input == "-":
        log_text = sys.stdin.read()
    else:
        log_text = Path(args.input).read_text(encoding="utf-8")

    started_at = _parse_iso_timestamp(args.started_at)
    if not started_at:
        started_at = datetime.datetime.now(datetime.timezone.utc).isoformat()

    writer = ReceiptWriter(log_text, started_at, args.exit_status)
    receipt = writer.build()
    receipt = _normalize_receipt(receipt)

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    if writer.incidents:
        print(f"verify-full receipt written with {len(writer.incidents)} incident(s):", file=sys.stderr)
        for incident in writer.incidents:
            print(f"  - {incident}", file=sys.stderr)
    else:
        print(f"verify-full receipt written to {output_path}", file=sys.stderr)

    return 0 if not writer.incidents else 1


if __name__ == "__main__":
    sys.exit(main())
