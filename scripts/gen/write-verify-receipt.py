#!/usr/bin/env python3
"""Write a machine-readable verify-full receipt from captured output.

This script parses the stdout of `python3 scripts/manager.py verify full` and
writes a structured JSON receipt with per-check and per-item identity.

The receipt schema is version 2: each check is a dict with aggregate counts and
a nested `items` list.  Identity coverage is recorded per check so consumers
know whether the `items` list is exhaustive (`full`), partial (`partial`), or
only an aggregate (`aggregate_only`).

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

RESULT_PASS = "pass"
RESULT_FAIL = "fail"
RESULT_SKIP = "skip"

# Canonical issue owners for each verify domain.
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

# Issue owner for per-item skip identities that belong to the diagnostic/T3
# compile-skip contract (#815).
SKIP_OWNER_ISSUE = "815"

# Identity coverage per domain.  `t3_wasm_validate` is upgraded to `full` when
# pass fixture identities are present in the report.
IDENTITY_COVERAGE = {
    "size": "aggregate_only",
    "quick_checks": "aggregate_only",
    "t3_wasm_validate": "nonpass_full",
    "fixture_parity": "nonpass_full",
    "wat_roundtrip": "partial",
    "component_interop": "full",
    "cli_parity": "full",
    "diag_parity": "full",
    "fixpoint": "aggregate_only",
}


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
        self.evidence: dict[str, str] = {}
        self.items: list[dict] = []

    def as_dict(self) -> dict:
        d = {
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
        if self.evidence:
            d["evidence"] = self.evidence
        return d


class _Section:
    """A contiguous block of the log belonging to a single domain."""

    def __init__(self, check_id: str, start_idx: int, end_idx: int | None = None):
        self.check_id = check_id
        self.start_idx = start_idx
        self.end_idx = end_idx


class ReceiptWriter:
    """Parse `verify full` output and emit a domain-aware receipt."""

    # Each section is identified by a header line. The end is the first line
    # matching the next section start or the end of the log.
    SECTION_STARTERS = [
        ("size", re.compile(r"^\[size\] Checking hello\.wasm binary size gate")),
        ("quick", re.compile(r"^\[bg\] Running background checks in parallel")),
        ("fixtures", re.compile(r"^\[fixtures\] Running selfhost fixture parity")),
        ("wat", re.compile(r"^\[wat\] Running WAT roundtrip")),
        ("component", re.compile(r"^\[component\] Component interop smoke test")),
        ("cli_parity", re.compile(r"^\[cli-parity\] Checking selfhost CLI surface")),
        ("diag_parity", re.compile(r"^\[diag-parity\] Checking .* diag: fixtures")),
        ("fixpoint", re.compile(r"^\[selfhost-fixpoint\] Fixpoint gate \(full verify\)")),
    ]

    # Aggregate summary patterns.
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
    TOTAL_CHECKS_RE = re.compile(r"^Total checks:\s+(\d+)")
    PASSED_RE = re.compile(r"^Passed:\s+(\d+)")
    SKIPPED_RE = re.compile(r"^Skipped:\s+(\d+)")
    FAILED_RE = re.compile(r"^Failed:\s+(\d+)")

    # Per-item identity patterns.
    PASS_ITEM_RE = re.compile(r"^\s*pass:\s+([^()]+?)(?:\s+\(|$)")
    FAIL_ITEM_RE = re.compile(r"^\s*FAIL:\s+([^()]+?)(?:\s+\(|$)")
    SKIP_ITEM_RE = re.compile(r"^\s*skip:\s+(\S+)")
    COMPONENT_PASS_RE = re.compile(r"\u2713\s+component\s+interop:\s+(\S+)")
    COMPONENT_FAIL_RE = re.compile(r"\u2717\s+component\s+interop:\s+(\S+)")
    SIZE_PASS_RE = re.compile(r"\u2713\s+hello\.wasm\s+binary\s+size:\s+(\d+)\s+bytes")
    SIZE_FAIL_RE = re.compile(r"\u2717\s+hello\.wasm\s+binary\s+size:")
    FIXPOINT_S2_RE = re.compile(r"sha256\(s2\)\s*=\s*([0-9a-f]+)")
    FIXPOINT_S3_RE = re.compile(r"sha256\(s3\)\s*=\s*([0-9a-f]+)")

    def __init__(
        self,
        log_text: str,
        *,
        started_at: str | None = None,
        exit_status: int = 0,
        verified_commit: str | None = None,
        t3_report: dict | None = None,
    ):
        self.lines = [self._clean(l) for l in log_text.splitlines()]
        self.started_at = started_at
        self.exit_status = exit_status
        self.verified_commit = verified_commit
        self.t3_report = self._resolve_t3_report(t3_report)
        self.sections = self._detect_sections()
        self.checks: dict[str, _Check] = {}
        self.incidents: list[str] = []

    def _resolve_t3_report(self, t3_report: dict | None) -> dict | None:
        """Use the provided report, or load it from the default build path."""
        if t3_report is not None:
            return t3_report
        report_path = REPO_ROOT / ".build" / "t3-wasm-validate-report.json"
        if report_path.is_file():
            try:
                return json.loads(report_path.read_text(encoding="utf-8"))
            except (json.JSONDecodeError, OSError):
                return None
        return None

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
        owner_issue: str | None = None,
        baseline_status: str = "existing",
        new_or_existing: str = "existing",
    ) -> None:
        check = self._make_check(check_id)
        item = {
            "check_id": check_id,
            "item_id": item_id.strip(),
            "result": result,
            "category": category,
            "owner_issue": owner_issue or check.owner_issue,
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
            if self.SIZE_PASS_RE.search(line):
                check.pass_count = 1
                check.result = RESULT_PASS
                return
            if self.SIZE_FAIL_RE.search(line):
                check.fail_count = 1
                check.result = RESULT_FAIL
                return
        check.skip_count = 1
        check.result = RESULT_SKIP

    def _parse_quick(self) -> None:
        section = self._section_lines("quick")
        check = self._make_check("quick_checks")
        check.command = "python3 scripts/manager.py verify quick"
        check.primary_path = "scripts/manager.py"

        for idx, line in enumerate(section):
            if self.TOTAL_CHECKS_RE.search(line):
                for nxt in section[idx + 1 : idx + 5]:
                    pm = self.PASSED_RE.search(nxt)
                    if pm:
                        check.pass_count = int(pm.group(1))
                    sm = self.SKIPPED_RE.search(nxt)
                    if sm:
                        check.skip_count = int(sm.group(1))
                    fm = self.FAILED_RE.search(nxt)
                    if fm:
                        check.fail_count = int(fm.group(1))
                break

        if check.fail_count:
            check.result = RESULT_FAIL
        else:
            check.result = RESULT_PASS if check.pass_count else RESULT_SKIP

    def _parse_t3(self) -> None:
        check = self._make_check("t3_wasm_validate")
        check.command = "python3 scripts/check/check-t3-wasm-validate.py"
        check.primary_path = "scripts/check/check-t3-wasm-validate.py"
        if self.t3_report is not None:
            self._parse_t3_from_report(check)
        else:
            self._parse_t3_from_log(check)

    def _parse_t3_from_report(self, check: _Check) -> None:
        report = self.t3_report or {}
        pass_count = int(report.get("pass_count", 0))
        fail_validate = int(report.get("fail_validate", 0))
        fail_compile = int(report.get("fail_compile", 0))
        skip_count = int(report.get("skip_count", 0))

        for item in report.get("items", []):
            if not isinstance(item, dict):
                continue
            status = item.get("status", "")
            fixture = item.get("fixture", "")
            detail = item.get("detail", "")
            if status == "pass":
                self._add_item(
                    "t3_wasm_validate",
                    fixture,
                    RESULT_PASS,
                    "t3-pass",
                    detail,
                )
            elif status == "validate-fail":
                self._add_item(
                    "t3_wasm_validate",
                    fixture,
                    RESULT_FAIL,
                    "t3-validate-failure",
                    detail,
                )
            elif status == "compile-fail":
                self._add_item(
                    "t3_wasm_validate",
                    fixture,
                    RESULT_FAIL,
                    "t3-compile-failure",
                    detail,
                )
            elif status == "timeout":
                self._add_item(
                    "t3_wasm_validate",
                    fixture,
                    RESULT_SKIP,
                    "t3-compile-timeout",
                    detail,
                    owner_issue=OWNER_ISSUES["t3_wasm_validate"],
                )
            elif status == "skip":
                self._add_item(
                    "t3_wasm_validate",
                    fixture,
                    RESULT_SKIP,
                    "t3-compile-skip",
                    detail,
                    owner_issue=SKIP_OWNER_ISSUE,
                )

        pass_items = sum(1 for i in check.items if i["result"] == RESULT_PASS)
        fail_items = sum(1 for i in check.items if i["result"] == RESULT_FAIL)
        skip_items = sum(1 for i in check.items if i["result"] == RESULT_SKIP)

        check.pass_count = pass_count
        check.fail_count = fail_validate + fail_compile
        check.skip_count = skip_count
        check.result = RESULT_FAIL if check.fail_count else RESULT_PASS

        if pass_items:
            check.identity_coverage = "full"
        elif fail_items or skip_items:
            check.identity_coverage = "nonpass_full"
        else:
            check.identity_coverage = "partial"

        if pass_items != pass_count:
            self.incidents.append(
                f"t3_wasm_validate: pass identity count ({pass_items}) != "
                f"pass_count ({pass_count})"
            )
        if fail_items != fail_validate + fail_compile:
            self.incidents.append(
                f"t3_wasm_validate: fail identity count ({fail_items}) != "
                f"fail_validate + fail_compile ({fail_validate + fail_compile})"
            )
        if skip_items != skip_count:
            self.incidents.append(
                f"t3_wasm_validate: skip identity count ({skip_items}) != "
                f"skip_count ({skip_count})"
            )

    def _parse_t3_from_log(self, check: _Check) -> None:
        section = self._section_lines("quick")
        pass_count = fail_validate = fail_compile = skip_count = 0
        for line in section:
            m = self.T3_SUMMARY_RE.search(line)
            if m:
                pass_count = int(m.group(1))
                fail_validate = int(m.group(2))
                fail_compile = int(m.group(3))
                skip_count = int(m.group(4))
                break

        check.pass_count = pass_count
        check.fail_count = fail_validate + fail_compile
        check.skip_count = skip_count
        check.result = RESULT_FAIL if check.fail_count else RESULT_PASS

        for line in section:
            if line.startswith("VALIDATE FAIL:"):
                fixture = line.split("—", 1)[0].replace("VALIDATE FAIL:", "").strip()
                detail = line.split("—", 1)[1].strip() if "—" in line else ""
                self._add_item(
                    "t3_wasm_validate",
                    fixture,
                    RESULT_FAIL,
                    "t3-validate-failure",
                    detail,
                )
            elif line.startswith("COMPILE FAIL:"):
                fixture = line.replace("COMPILE FAIL:", "").strip()
                self._add_item(
                    "t3_wasm_validate",
                    fixture,
                    RESULT_FAIL,
                    "t3-compile-failure",
                    "compile failed",
                )
            elif line.startswith("COMPILE TIMEOUT:"):
                fixture = line.replace("COMPILE TIMEOUT:", "").strip()
                self._add_item(
                    "t3_wasm_validate",
                    fixture,
                    RESULT_SKIP,
                    "t3-compile-timeout",
                    "compile timeout",
                    owner_issue=OWNER_ISSUES["t3_wasm_validate"],
                )

        # When the report is unavailable, the log is truncated and we only have
        # partial identity; leave the aggregate counts from the summary and let
        # the partial-coverage invariant validate the fail identities.
        check.identity_coverage = "partial"

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
                    detail = line.split("(", 1)[1].rstrip(")") if "(" in line else None
                    self._add_item(
                        "fixture_parity",
                        fixture,
                        RESULT_FAIL,
                        "fixture-parity",
                        detail,
                    )
            elif line.startswith("  skip:"):
                m = self.SKIP_ITEM_RE.match(line)
                if m:
                    fixture = m.group(1).strip()
                    detail = line.split("(", 1)[1].rstrip(")") if "(" in line else None
                    self._add_item(
                        "fixture_parity",
                        fixture,
                        RESULT_SKIP,
                        "fixture-parity",
                        detail,
                    )

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
                m = self.FAIL_ITEM_RE.match(line)
                if m:
                    label = m.group(1).strip()
                    detail = line.split("(", 1)[1].rstrip(")") if "(" in line else None
                    self._add_item(
                        "wat_roundtrip",
                        label,
                        RESULT_FAIL,
                        "wat-roundtrip",
                        detail,
                    )

    def _parse_component(self) -> None:
        section = self._section_lines("component")
        check = self._make_check("component_interop")
        check.command = "python3 scripts/manager.py verify component"
        check.primary_path = "tests/component-interop/"

        pass_count = fail_count = skip_count = 0
        for line in section:
            if self.COMPONENT_PASS_RE.search(line):
                pass_count += 1
                m = self.COMPONENT_PASS_RE.search(line)
                if m:
                    self._add_item(
                        "component_interop",
                        m.group(1),
                        RESULT_PASS,
                        "component-interop",
                    )
            elif self.COMPONENT_FAIL_RE.search(line):
                fail_count += 1
                m = self.COMPONENT_FAIL_RE.search(line)
                if m:
                    self._add_item(
                        "component_interop",
                        m.group(1),
                        RESULT_FAIL,
                        "component-interop",
                    )
            elif "\u2299 component interop" in line:
                # Real skip: "⊙ component interop (wasmtime not found) (skipped)"
                skip_count += 1
                detail = ""
                if "wasmtime not found" in line:
                    item_id = "wasmtime-not-found"
                    detail = "wasmtime not found"
                elif "scripts not found" in line:
                    item_id = "scripts-not-found"
                    detail = "component interop scripts not found"
                else:
                    item_id = "component-interop"
                self._add_item(
                    "component_interop",
                    item_id,
                    RESULT_SKIP,
                    "component-interop",
                    detail,
                )

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
            if line.startswith("  pass:"):
                m = self.PASS_ITEM_RE.match(line)
                if m:
                    item = m.group(1).strip()
                    self._add_item(
                        "cli_parity",
                        item,
                        RESULT_PASS,
                        "cli-parity",
                    )
            elif line.startswith("  FAIL:"):
                m = self.FAIL_ITEM_RE.match(line)
                if m:
                    item = m.group(1).strip()
                    detail = line.split("(", 1)[1].rstrip(")") if "(" in line else None
                    self._add_item(
                        "cli_parity",
                        item,
                        RESULT_FAIL,
                        "cli-parity",
                        detail,
                    )

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
            if line.startswith("  pass:"):
                m = self.PASS_ITEM_RE.match(line)
                if m:
                    fixture = m.group(1).strip()
                    self._add_item(
                        "diag_parity",
                        fixture,
                        RESULT_PASS,
                        "diag-parity",
                    )
            elif line.startswith("  FAIL:"):
                m = self.FAIL_ITEM_RE.match(line)
                if m:
                    fixture = m.group(1).strip()
                    detail = line.split("(", 1)[1].rstrip(")") if "(" in line else None
                    self._add_item(
                        "diag_parity",
                        fixture,
                        RESULT_FAIL,
                        "diag-parity",
                        detail,
                    )
            elif line.startswith("  skip:"):
                m = self.SKIP_ITEM_RE.match(line)
                if m:
                    fixture = m.group(1).strip()
                    detail = line.split("(", 1)[1].rstrip(")") if "(" in line else None
                    self._add_item(
                        "diag_parity",
                        fixture,
                        RESULT_SKIP,
                        "diag-parity",
                        detail,
                        owner_issue=SKIP_OWNER_ISSUE,
                    )

    def _parse_fixpoint(self) -> None:
        section = self._section_lines("fixpoint")
        check = self._make_check("fixpoint")
        check.command = "python3 scripts/manager.py selfhost fixpoint"
        check.primary_path = "scripts/run/arukellt-selfhost.sh"
        check.identity_coverage = "aggregate_only"

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
            if "selfhost fixpoint reached" in line or "\u2713 selfhost fixpoint reached" in line:
                reached = True

        if s2_hash:
            check.evidence["s2_hash"] = s2_hash
        if s3_hash:
            check.evidence["s3_hash"] = s3_hash

        if reached:
            check.pass_count = 1
            check.result = RESULT_PASS
        else:
            check.fail_count = 1
            check.result = RESULT_FAIL

    def _ensure_invariants(self) -> None:
        """Verify identity/aggregate invariants and record incidents."""
        for check in self.checks.values():
            pass_items = sum(1 for i in check.items if i["result"] == RESULT_PASS)
            fail_items = sum(1 for i in check.items if i["result"] == RESULT_FAIL)
            skip_items = sum(1 for i in check.items if i["result"] == RESULT_SKIP)

            if check.identity_coverage == "full":
                if pass_items != check.pass_count:
                    self.incidents.append(
                        f"{check.check_id}: pass identity count ({pass_items}) != "
                        f"pass_count ({check.pass_count})"
                    )
                if fail_items != check.fail_count:
                    self.incidents.append(
                        f"{check.check_id}: fail identity count ({fail_items}) != "
                        f"fail_count ({check.fail_count})"
                    )
                if skip_items != check.skip_count:
                    self.incidents.append(
                        f"{check.check_id}: skip identity count ({skip_items}) != "
                        f"skip_count ({check.skip_count})"
                    )
            elif check.identity_coverage == "nonpass_full":
                if fail_items != check.fail_count:
                    self.incidents.append(
                        f"{check.check_id}: fail identity count ({fail_items}) != "
                        f"fail_count ({check.fail_count})"
                    )
                if skip_items != check.skip_count:
                    self.incidents.append(
                        f"{check.check_id}: skip identity count ({skip_items}) != "
                        f"skip_count ({check.skip_count})"
                    )
                if pass_items != 0:
                    self.incidents.append(
                        f"{check.check_id}: nonpass_full coverage has "
                        f"{pass_items} pass item(s)"
                    )
            elif check.identity_coverage == "partial":
                if fail_items != check.fail_count:
                    self.incidents.append(
                        f"{check.check_id}: fail identity count ({fail_items}) != "
                        f"fail_count ({check.fail_count})"
                    )
            elif check.identity_coverage == "aggregate_only":
                if check.items:
                    self.incidents.append(
                        f"{check.check_id}: aggregate_only check has "
                        f"{len(check.items)} item(s)"
                    )

    def build(self) -> dict:
        """Parse the log and produce the receipt dictionary."""
        self._parse_size()
        self._parse_quick()
        self._parse_t3()
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
        item_failures = sum(
            1 for c in self.checks.values() for i in c.items if i["result"] == RESULT_FAIL
        )
        item_skips = sum(
            1 for c in self.checks.values() for i in c.items if i["result"] == RESULT_SKIP
        )

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
            "verified_commit": self.verified_commit or _get_git_commit(),
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
    order = [
        "schema_version",
        "generated_at",
        "started_at",
        "finished_at",
        "duration_seconds",
        "verified_commit",
        "exit_status",
        "status",
        "log_sha256",
        "summary",
        "checks",
    ]
    return {k: receipt[k] for k in order if k in receipt}


def build_receipt(
    log_text: str,
    *,
    started_at: str | None = None,
    exit_status: int = 0,
    verified_commit: str | None = None,
    t3_report: dict | None = None,
) -> dict:
    """Parse `verify full` output and return a schema v2 receipt dict.

    Parameters
    ----------
    log_text: captured stdout of `python3 scripts/manager.py verify full`.
    started_at: optional ISO 8601 timestamp when the verify run started.
    exit_status: shell exit code of the verify command.
    verified_commit: commit being verified; falls back to current HEAD.
    t3_report: optional dict from `check-t3-wasm-validate.py`.  If omitted and
        `.build/t3-wasm-validate-report.json` exists, it is loaded.
    """
    parsed_started_at = _parse_iso_timestamp(started_at)
    if not parsed_started_at:
        parsed_started_at = datetime.datetime.now(datetime.timezone.utc).isoformat()

    writer = ReceiptWriter(
        log_text,
        started_at=parsed_started_at,
        exit_status=exit_status,
        verified_commit=verified_commit,
        t3_report=t3_report,
    )
    return writer.build()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Write a verify-full receipt from captured output")
    parser.add_argument("--input", "-i", default="-", help="Input log file (default: stdin)")
    parser.add_argument("--output", "-o", required=True, help="Output JSON receipt path")
    parser.add_argument("--started-at", help="ISO 8601 timestamp when verify full started")
    parser.add_argument(
        "--exit-status", type=int, default=0, help="Shell exit status of the verify command"
    )
    parser.add_argument(
        "--verified-commit", help="Commit hash being verified (defaults to current HEAD)"
    )
    args = parser.parse_args(argv)

    if args.input == "-":
        log_text = sys.stdin.read()
    else:
        log_text = Path(args.input).read_text(encoding="utf-8")

    receipt = build_receipt(
        log_text,
        started_at=args.started_at,
        exit_status=args.exit_status,
        verified_commit=args.verified_commit,
    )
    receipt = _normalize_receipt(receipt)

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )

    if receipt["summary"]["incidents"]:
        print(
            f"verify-full receipt written with {len(receipt['summary']['incidents'])} incident(s):",
            file=sys.stderr,
        )
        for incident in receipt["summary"]["incidents"]:
            print(f"  - {incident}", file=sys.stderr)
    else:
        print(f"verify-full receipt written to {output_path}", file=sys.stderr)

    return 0 if not receipt["summary"]["incidents"] else 1


if __name__ == "__main__":
    sys.exit(main())
