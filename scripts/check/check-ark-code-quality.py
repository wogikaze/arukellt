#!/usr/bin/env python3
"""Ark compiler source quality gates and debt inventory.

Hard fails (always):
  - tab characters in src/compiler/**/*.ark
  - leading whitespace >= MAX_LEADING_WS on a non-empty line

Ratchet fails (vs docs/data/ark-code-quality-baseline.toml):
  - count of lines with length >= MAX_LINE_LEN_HARD
  - count of thin forwarding wrappers
  - count of files containing exactly one function

Usage:
  python3 scripts/check/check-ark-code-quality.py
  python3 scripts/check/check-ark-code-quality.py --report
  python3 scripts/check/check-ark-code-quality.py --write-baseline --issue 123
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from quality.metrics import scan_ark_source  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[2]
COMPILER_ROOT = REPO_ROOT / "src" / "compiler"
BASELINE_PATH = REPO_ROOT / "docs" / "data" / "ark-code-quality-baseline.toml"

MAX_LEADING_WS = 64
MAX_LINE_LEN_HARD = 200

@dataclass
class Inventory:
    tab_files: list[str] = field(default_factory=list)
    extreme_indent: list[str] = field(default_factory=list)
    long_lines: list[str] = field(default_factory=list)
    thin_wrappers: list[str] = field(default_factory=list)
    single_function_files: list[str] = field(default_factory=list)

    @property
    def counts(self) -> dict[str, int]:
        return {
            "tabs_files": len(self.tab_files),
            "extreme_indent_lines": len(self.extreme_indent),
            "lines_ge_200": len(self.long_lines),
            "thin_wrappers": len(self.thin_wrappers),
            "single_function_files": len(self.single_function_files),
        }


def _rel(path: Path) -> str:
    return str(path.relative_to(REPO_ROOT)).replace("\\", "/")


def scan_paths(paths: list[Path]) -> Inventory:
    inv = Inventory()
    for path in sorted(paths):
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        rel = _rel(path)
        if "\t" in text:
            inv.tab_files.append(rel)
        for line_no, line in enumerate(text.splitlines(), 1):
            if not line.strip():
                continue
            stripped = line.lstrip(" \t")
            lead = len(line) - len(stripped)
            if lead >= MAX_LEADING_WS:
                inv.extreme_indent.append(f"{rel}:{line_no}: leading_ws={lead}")
            if len(line) >= MAX_LINE_LEN_HARD:
                inv.long_lines.append(f"{rel}:{line_no}: len={len(line)}")
        _, functions = scan_ark_source(rel, text)
        inv.thin_wrappers.extend(
            f"{rel}:{item.line}: {item.symbol}"
            for item in functions
            if item.is_thin_wrapper
        )
        if len(functions) == 1:
            inv.single_function_files.append(rel)
    return inv


def scan_compiler() -> Inventory:
    return scan_paths(list(COMPILER_ROOT.rglob("*.ark")))


def _changed_compiler_paths(base: str) -> list[Path]:
    result = subprocess.run(
        ["git", "diff", "--name-only", "--diff-filter=ACMR", base, "--", "src/compiler/**/*.ark"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    paths = []
    for rel in result.stdout.splitlines():
        path = REPO_ROOT / rel
        if path.is_file() and path.suffix == ".ark":
            paths.append(path)
    return sorted(paths)


def _base_inventory(path: Path, base: str) -> Inventory:
    rel = _rel(path)
    result = subprocess.run(
        ["git", "show", f"{base}:{rel}"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        return Inventory()
    inv = Inventory()
    text = result.stdout
    if "\t" in text:
        inv.tab_files.append(rel)
    for line_no, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        stripped = line.lstrip(" \t")
        lead = len(line) - len(stripped)
        if lead >= MAX_LEADING_WS:
            inv.extreme_indent.append(f"{rel}:{line_no}: leading_ws={lead}")
        if len(line) >= MAX_LINE_LEN_HARD:
            inv.long_lines.append(f"{rel}:{line_no}: len={len(line)}")
    _, functions = scan_ark_source(rel, text)
    inv.thin_wrappers.extend(
        f"{rel}:{item.line}: {item.symbol}"
        for item in functions
        if item.is_thin_wrapper
    )
    if len(functions) == 1:
        inv.single_function_files.append(rel)
    return inv


def check_changed(base: str) -> int:
    failures: list[str] = []
    paths = _changed_compiler_paths(base)
    for path in paths:
        current = scan_paths([path]).counts
        previous = _base_inventory(path, base).counts
        for key in (
            "tabs_files",
            "extreme_indent_lines",
            "lines_ge_200",
            "thin_wrappers",
            "single_function_files",
        ):
            if current[key] > previous[key]:
                failures.append(
                    f"touched-code regression: {_rel(path)} {key} "
                    f"{current[key]} > {previous[key]} ({base})"
                )
    if failures:
        print("FAIL:")
        for failure in failures:
            print(f"  - {failure}")
        return 1
    print(f"PASS: touched-code quality ratchet ({len(paths)} files vs {base})")
    return 0


def _parse_baseline(path: Path) -> dict[str, int]:
    if not path.is_file():
        return {}
    counts: dict[str, int] = {}
    in_counts = False
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line == "[counts]":
            in_counts = True
            continue
        if line.startswith("["):
            in_counts = False
            continue
        if in_counts and "=" in line:
            key, value = [p.strip() for p in line.split("=", 1)]
            counts[key] = int(value)
    return counts


def _tracking_issue_exists(issue: int) -> bool:
    pattern = f"{issue}-*.md"
    return any(
        any((REPO_ROOT / state).glob(pattern))
        for state in ("issues/open", "issues/done")
    )


def _write_baseline(path: Path, inv: Inventory, issue: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    counts = inv.counts
    body = (
        "# Auto-maintained inventory ceilings for Ark compiler source quality.\n"
        "# Lower counts via remediation; never raise without an explicit decision.\n"
        "# Generated/updated by scripts/check/check-ark-code-quality.py --write-baseline --issue N\n"
        "\n"
        "owner = \"compiler-tooling\"\n"
        "increase_requires_tracking_issue = true\n"
        f"last_update_issue = {issue}\n"
        "\n"
        f"max_leading_ws = {MAX_LEADING_WS}\n"
        f"max_line_len_hard = {MAX_LINE_LEN_HARD}\n"
        "\n"
        "[counts]\n"
        f"tabs_files = {counts['tabs_files']}\n"
        f"extreme_indent_lines = {counts['extreme_indent_lines']}\n"
        f"lines_ge_200 = {counts['lines_ge_200']}\n"
        f"thin_wrappers = {counts['thin_wrappers']}\n"
        f"single_function_files = {counts['single_function_files']}\n"
    )
    path.write_text(body, encoding="utf-8")


def _print_sample(title: str, items: list[str], limit: int = 20) -> None:
    print(f"\n{title} ({len(items)})")
    for item in items[:limit]:
        print(f"  {item}")
    if len(items) > limit:
        print(f"  ... and {len(items) - limit} more")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--report",
        action="store_true",
        help="Print inventory and always exit 0 (advisory).",
    )
    parser.add_argument(
        "--write-baseline",
        action="store_true",
        help="Rewrite docs/data/ark-code-quality-baseline.toml from current inventory.",
    )
    parser.add_argument(
        "--issue",
        type=int,
        help="Tracking issue required by --write-baseline.",
    )
    parser.add_argument(
        "--changed",
        action="store_true",
        help="Reject new findings in compiler Ark files changed from --base.",
    )
    parser.add_argument("--base", default="HEAD", help="Git base for --changed (default: HEAD).")
    args = parser.parse_args()

    if args.changed:
        return check_changed(args.base)

    inv = scan_compiler()
    counts = inv.counts
    print("ark code quality inventory:")
    for key, value in counts.items():
        print(f"  {key}: {value}")

    if args.write_baseline:
        if args.issue is None:
            parser.error("--write-baseline requires --issue")
        if not _tracking_issue_exists(args.issue):
            parser.error(f"tracking issue {args.issue} does not exist in issues/open or issues/done")
        _write_baseline(BASELINE_PATH, inv, args.issue)
        print(f"wrote baseline: {_rel(BASELINE_PATH)}")
        return 0

    if args.report:
        _print_sample("tab files", inv.tab_files)
        _print_sample("extreme indent", inv.extreme_indent)
        _print_sample("lines >= 200", inv.long_lines)
        _print_sample("thin wrappers", inv.thin_wrappers)
        _print_sample("single-function files", inv.single_function_files)
        return 0

    failures: list[str] = []

    if counts["tabs_files"] > 0:
        failures.append(f"tabs forbidden: {counts['tabs_files']} files")
        _print_sample("tab files", inv.tab_files)
    if counts["extreme_indent_lines"] > 0:
        failures.append(
            f"extreme indent (>= {MAX_LEADING_WS}): {counts['extreme_indent_lines']} lines"
        )
        _print_sample("extreme indent", inv.extreme_indent)

    baseline = _parse_baseline(BASELINE_PATH)
    if not baseline:
        failures.append(
            f"missing baseline {_rel(BASELINE_PATH)}; run with --write-baseline after remediation"
        )
    else:
        for key in ("lines_ge_200", "thin_wrappers", "single_function_files"):
            current = counts[key]
            ceiling = baseline.get(key)
            if ceiling is None:
                failures.append(f"baseline missing key: {key}")
                continue
            if current > ceiling:
                failures.append(f"ratchet regression: {key} {current} > baseline {ceiling}")
                if key == "lines_ge_200":
                    _print_sample("lines >= 200", inv.long_lines)
                elif key == "thin_wrappers":
                    _print_sample("thin wrappers", inv.thin_wrappers)
                else:
                    _print_sample("single-function files", inv.single_function_files)

    if failures:
        print("\nFAIL:")
        for item in failures:
            print(f"  - {item}")
        return 1

    print("PASS: ark code quality gates")
    return 0


if __name__ == "__main__":
    sys.exit(main())
