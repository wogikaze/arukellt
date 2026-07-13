#!/usr/bin/env python3
"""Ark compiler source quality gates (tabs, indent, long lines, thin wrappers).

Hard fails (always):
  - tab characters in src/compiler/**/*.ark
  - leading whitespace >= MAX_LEADING_WS on a non-empty line

Ratchet fails (vs docs/data/ark-code-quality-baseline.toml):
  - count of lines with length >= MAX_LINE_LEN_HARD
  - count of thin forwarding wrappers

Usage:
  python3 scripts/check/check-ark-code-quality.py
  python3 scripts/check/check-ark-code-quality.py --report
  python3 scripts/check/check-ark-code-quality.py --write-baseline
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
COMPILER_ROOT = REPO_ROOT / "src" / "compiler"
BASELINE_PATH = REPO_ROOT / "docs" / "data" / "ark-code-quality-baseline.toml"

MAX_LEADING_WS = 64
MAX_LINE_LEN_HARD = 200

# Thin wrapper: function body is a single call (optional return) that forwards
# the same argument names (order-preserving) to another symbol.
FN_HEAD_RE = re.compile(
    r"^(?:pub\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)\s*(?:->\s*[^{]+)?\s*\{?\s*$"
)
CALL_RE = re.compile(
    r"^(?:return\s+)?([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)\s*\((.*)\)\s*;?\s*$"
)


@dataclass
class Inventory:
    tab_files: list[str] = field(default_factory=list)
    extreme_indent: list[str] = field(default_factory=list)
    long_lines: list[str] = field(default_factory=list)
    thin_wrappers: list[str] = field(default_factory=list)

    @property
    def counts(self) -> dict[str, int]:
        return {
            "tabs_files": len(self.tab_files),
            "extreme_indent_lines": len(self.extreme_indent),
            "lines_ge_200": len(self.long_lines),
            "thin_wrappers": len(self.thin_wrappers),
        }


def _rel(path: Path) -> str:
    return str(path.relative_to(REPO_ROOT)).replace("\\", "/")


def _param_names(params: str) -> list[str]:
    names: list[str] = []
    for part in params.split(","):
        part = part.strip()
        if not part:
            continue
        name = part.split(":")[0].strip()
        if name.startswith("mut "):
            name = name[4:].strip()
        if name:
            names.append(name)
    return names


def _arg_names(args: str) -> list[str]:
    names: list[str] = []
    depth = 0
    current: list[str] = []
    for ch in args:
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth = max(0, depth - 1)
        if ch == "," and depth == 0:
            token = "".join(current).strip()
            if token:
                names.append(token.split(".")[0].strip())
            current = []
            continue
        current.append(ch)
    token = "".join(current).strip()
    if token:
        names.append(token.split(".")[0].strip())
    return names


def _strip_line_comment(line: str) -> str:
    in_string = False
    quote = ""
    i = 0
    while i < len(line):
        ch = line[i]
        if in_string:
            if ch == "\\" and i + 1 < len(line):
                i += 2
                continue
            if ch == quote:
                in_string = False
            i += 1
            continue
        if ch in "\"'":
            in_string = True
            quote = ch
            i += 1
            continue
        if ch == "/" and i + 1 < len(line) and line[i + 1] == "/":
            return line[:i].rstrip()
        i += 1
    return line.rstrip()


def _find_thin_wrappers(path: Path, text: str) -> list[str]:
    findings: list[str] = []
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        raw = lines[i]
        stripped = _strip_line_comment(raw).strip()
        match = FN_HEAD_RE.match(stripped)
        if not match:
            i += 1
            continue
        fn_name = match.group(1)
        params = _param_names(match.group(2))
        body_lines: list[str] = []
        # Opening brace may be on this line or the next.
        opened = "{" in stripped
        j = i + 1
        if not opened:
            while j < len(lines) and not lines[j].strip():
                j += 1
            if j >= len(lines) or "{" not in lines[j]:
                i += 1
                continue
            j += 1
        depth = 1
        while j < len(lines) and depth > 0:
            line = lines[j]
            code = _strip_line_comment(line)
            depth += code.count("{") - code.count("}")
            if depth > 0:
                body = code.strip()
                if body and body != "}":
                    body_lines.append(body.rstrip(";").strip())
            j += 1
        if len(body_lines) == 1:
            call = CALL_RE.match(body_lines[0])
            if call:
                callee = call.group(1)
                # Skip self-recursive / trivial field returns: require a call-like callee.
                if "(" in body_lines[0] and callee != fn_name:
                    args = _arg_names(call.group(2))
                    # Exact forward of parameter list (same names, same order).
                    if args == params:
                        findings.append(f"{_rel(path)}:{i + 1}: {fn_name} -> {callee}")
        i = j if j > i else i + 1
    return findings


def scan_compiler() -> Inventory:
    inv = Inventory()
    for path in sorted(COMPILER_ROOT.rglob("*.ark")):
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
        inv.thin_wrappers.extend(_find_thin_wrappers(path, text))
    return inv


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


def _write_baseline(path: Path, inv: Inventory) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    counts = inv.counts
    body = (
        "# Auto-maintained inventory ceilings for Ark compiler source quality.\n"
        "# Lower counts via remediation; never raise without an explicit decision.\n"
        "# Generated/updated by scripts/check/check-ark-code-quality.py --write-baseline\n"
        "\n"
        f"max_leading_ws = {MAX_LEADING_WS}\n"
        f"max_line_len_hard = {MAX_LINE_LEN_HARD}\n"
        "\n"
        "[counts]\n"
        f"tabs_files = {counts['tabs_files']}\n"
        f"extreme_indent_lines = {counts['extreme_indent_lines']}\n"
        f"lines_ge_200 = {counts['lines_ge_200']}\n"
        f"thin_wrappers = {counts['thin_wrappers']}\n"
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
    args = parser.parse_args()

    inv = scan_compiler()
    counts = inv.counts
    print("ark code quality inventory:")
    for key, value in counts.items():
        print(f"  {key}: {value}")

    if args.write_baseline:
        _write_baseline(BASELINE_PATH, inv)
        print(f"wrote baseline: {_rel(BASELINE_PATH)}")
        return 0

    if args.report:
        _print_sample("tab files", inv.tab_files)
        _print_sample("extreme indent", inv.extreme_indent)
        _print_sample("lines >= 200", inv.long_lines)
        _print_sample("thin wrappers", inv.thin_wrappers)
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
        for key in ("lines_ge_200", "thin_wrappers"):
            current = counts[key]
            ceiling = baseline.get(key)
            if ceiling is None:
                failures.append(f"baseline missing key: {key}")
                continue
            if current > ceiling:
                failures.append(f"ratchet regression: {key} {current} > baseline {ceiling}")
                if key == "lines_ge_200":
                    _print_sample("lines >= 200", inv.long_lines)
                else:
                    _print_sample("thin wrappers", inv.thin_wrappers)

    if failures:
        print("\nFAIL:")
        for item in failures:
            print(f"  - {item}")
        return 1

    print("PASS: ark code quality gates")
    return 0


if __name__ == "__main__":
    sys.exit(main())
