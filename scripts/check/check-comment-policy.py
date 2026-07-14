#!/usr/bin/env python3
"""Check structured comments and derived Ark public API documentation contracts."""

from __future__ import annotations

import argparse
import json
import re
from dataclasses import asdict, dataclass
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 in the bootstrap environment
    import tomli as tomllib


ROOT = Path(__file__).resolve().parents[2]
SCAN_ROOTS = (ROOT / "src/compiler", ROOT / "std", ROOT / "scripts")
TODO_RE = re.compile(r"\b(?:TODO|FIXME)\b")
STRUCTURED_RE = re.compile(
    r"\b(?:TODO|FIXME)\(#\d+ owner=[^\s)]+ "
    r"removal=(?:\"[^\"]+\"|'[^']+'|[^\s)]+) recheck=\d{4}-\d{2}-\d{2}\)"
)
PUB_FN_RE = re.compile(r"^\s*pub\s+fn\s+")
ITEM_RE = re.compile(r"^\s*(?:pub\s+)?(?:fn|struct|enum|trait|impl)\b")
VARIANT_RE = re.compile(r"^\s*[A-Z][A-Za-z0-9_]*(?:\s*\(|\s*,?\s*$)")
ISSUE_ONLY_RE = re.compile(
    r"^\s*(?://|#)\s*(?:issue\s*)?#\d+(?:\s+(?:in-file tests?|bulk))?\s*$",
    re.IGNORECASE,
)
COMMENTED_CODE_RE = re.compile(
    r"^\s*//\s*(?:pub\s+)?(?:fn\s+\w+\s*\([^)]*\)\s*(?:->[^\{]+)?\{|"
    r"let\s+(?:mut\s+)?\w+\s*=.+|use\s+[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)+\s*$|"
    r"if\s+.+\{|while\s+.+\{)"
)
BOILERPLATE_RE = re.compile(r"^//\s*Arukellt(?:\s+Selfhost|\s+Analysis|\s+LSP)?\s*[-—:]", re.I)


@dataclass(frozen=True)
class Finding:
    rule_id: str
    severity: str
    path: str
    line: int
    message: str


@dataclass(frozen=True)
class ApiSummary:
    external: int
    external_documented: int
    subsystem_boundary: int
    subsystem_boundary_documented: int
    internal_cross_module: int


def _comment_text(path: Path, line: str) -> str:
    if path.suffix == ".ark":
        comment_at = line.find("//")
        return line[comment_at:] if comment_at >= 0 else ""
    return line if line.lstrip().startswith("#") else ""


def _is_subsystem_boundary(path: Path) -> bool:
    return path.suffix == ".ark" and path.parent == ROOT / "src/compiler"


def _manifest_api_summary() -> tuple[int, int]:
    manifest = tomllib.loads((ROOT / "std/manifest.toml").read_text(encoding="utf-8"))
    functions = manifest.get("functions")
    if not isinstance(functions, list):
        raise ValueError("std/manifest.toml: functions must be an array")
    documented = sum(
        isinstance(entry, dict)
        and isinstance(entry.get("name"), str)
        and bool(entry.get("doc_category"))
        for entry in functions
    )
    return len(functions), documented


def collect_findings() -> tuple[list[Finding], ApiSummary]:
    findings: list[Finding] = []
    boundary_count = 0
    boundary_documented = 0
    internal_count = 0

    for scan_root in SCAN_ROOTS:
        for path in sorted(scan_root.rglob("*")):
            if not path.is_file() or path.suffix not in {".ark", ".py", ".sh"}:
                continue
            if path.name == "checks_broken.py":
                continue
            try:
                lines = path.read_text(encoding="utf-8").splitlines()
            except UnicodeDecodeError:
                continue
            rel = path.relative_to(ROOT).as_posix()
            for index, line in enumerate(lines):
                line_no = index + 1
                comment = _comment_text(path, line)
                if TODO_RE.search(comment) and not STRUCTURED_RE.search(comment):
                    findings.append(Finding("CQ-DOC-001", "error", rel, line_no, "unstructured TODO/FIXME"))
                if ISSUE_ONLY_RE.match(line):
                    findings.append(Finding("CQ-DOC-003", "error", rel, line_no, "issue-only comment"))
                if path.suffix == ".ark" and COMMENTED_CODE_RE.match(line):
                    findings.append(Finding("CQ-DOC-004", "error", rel, line_no, "commented-out production code"))
                if path.suffix == ".ark" and BOILERPLATE_RE.match(line):
                    findings.append(Finding("CQ-DOC-005", "warning", rel, line_no, "boilerplate header candidate"))
                if path.suffix == ".ark" and line.lstrip().startswith("///"):
                    next_line = lines[index + 1] if index + 1 < len(lines) else ""
                    if (
                        not next_line.lstrip().startswith("///")
                        and not ITEM_RE.match(next_line)
                        and not VARIANT_RE.match(next_line)
                    ):
                        findings.append(Finding("CQ-DOC-006", "error", rel, line_no, "doc comment is not attached to an item"))
                if path.suffix != ".ark" or not PUB_FN_RE.match(line):
                    continue
                documented = index > 0 and lines[index - 1].lstrip().startswith("///")
                if _is_subsystem_boundary(path):
                    boundary_count += 1
                    boundary_documented += int(documented)
                    if not documented:
                        findings.append(Finding("CQ-API-001", "error", rel, line_no, "stable subsystem boundary lacks a doc contract"))
                elif path.is_relative_to(ROOT / "src/compiler"):
                    internal_count += 1

    external, external_documented = _manifest_api_summary()
    if external_documented != external:
        findings.append(
            Finding(
                "CQ-API-001",
                "error",
                "std/manifest.toml",
                1,
                f"external API documentation coverage is {external_documented}/{external}",
            )
        )
    findings.sort(key=lambda item: (item.path, item.line, item.rule_id, item.message))
    return findings, ApiSummary(
        external,
        external_documented,
        boundary_count,
        boundary_documented,
        internal_count,
    )


def _report(findings: list[Finding], api: ApiSummary) -> dict[str, object]:
    errors = sum(item.severity == "error" for item in findings)
    warnings = sum(item.severity == "warning" for item in findings)
    return {
        "schema_version": 1,
        "status": "fail" if errors else "pass",
        "summary": {"errors": errors, "warnings": warnings},
        "api_classification": asdict(api),
        "findings": [asdict(item) for item in findings],
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    try:
        findings, api = collect_findings()
    except (OSError, ValueError, tomllib.TOMLDecodeError) as error:
        print(f"comment policy: ERROR: {error}")
        return 1
    report = _report(findings, api)
    if args.json:
        print(json.dumps(report, sort_keys=True))
    else:
        for finding in findings:
            print(f"{finding.rule_id} {finding.severity}: {finding.path}:{finding.line}: {finding.message}")
        print(
            "API A/B/C: "
            f"A={api.external} docs={api.external_documented}; "
            f"B={api.subsystem_boundary} docs={api.subsystem_boundary_documented}; "
            f"C={api.internal_cross_module}"
        )
        print(f"comment policy: {report['status'].upper()}")
    return 1 if report["status"] == "fail" else 0


if __name__ == "__main__":
    raise SystemExit(main())
