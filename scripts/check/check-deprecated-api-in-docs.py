#!/usr/bin/env python3
"""H8: Scan user-facing docs for deprecated API usage without a migration note.

This gate checks code blocks in quickstart.md, cookbook.md, and guide.md for
deprecated API names (from std/manifest.toml stability=deprecated). Each
occurrence must either:
  1. Be inside a code block with an explicit "deprecated" comment, OR
  2. Have a nearby API note explaining the deprecated status.

The gate is informational (exit 0) unless --strict is passed, in which case
undocumented deprecated API usage in quickstart.md fails the gate.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib

ROOT = Path(__file__).resolve().parents[2]

# Files to scan — user-facing docs with runnable code examples.
SCAN_FILES = [
    ROOT / "docs/quickstart.md",
    ROOT / "docs/stdlib/cookbook.md",
    ROOT / "docs/language/guide.md",
]

# Files exempt from strict mode (cookbook intentionally shows legacy patterns)
STRICT_EXEMPT = {ROOT / "docs/stdlib/cookbook.md", ROOT / "docs/language/guide.md"}


def load_deprecated_apis() -> set[str]:
    """Return the set of deprecated API names from std/manifest.toml."""
    with open(ROOT / "std/manifest.toml", "rb") as f:
        manifest = tomllib.load(f)
    return {fn["name"] for fn in manifest.get("functions", []) if fn.get("stability") == "deprecated"}


def extract_code_blocks(text: str) -> list[tuple[int, int, str]]:
    """Extract fenced code blocks. Returns list of (start_line, end_line, content).."""
    lines = text.split("\n")
    blocks = []
    in_block = False
    start = 0
    content_lines = []
    for i, line in enumerate(lines):
        if line.strip().startswith("```"):
            if in_block:
                blocks.append((start, i, "\n".join(content_lines)))
                in_block = False
                content_lines = []
            else:
                in_block = True
                start = i + 1
                content_lines = []
        elif in_block:
            content_lines.append(line)
    return blocks


def has_deprecation_marker(content: str) -> bool:
    """Check if a code block has a deprecation comment or note."""
    markers = ["deprecated", "DEPRECATED", "legacy", "Legacy", "migrat"]
    return any(m in content for m in markers)


def scan_file(path: Path, deprecated_apis: set[str]) -> list[dict]:
    """Scan a file for deprecated API usage in code blocks."""
    text = path.read_text(encoding="utf-8")
    blocks = extract_code_blocks(text)
    findings = []
    for start, end, content in blocks:
        # Skip non-ark code blocks
        # Find the fence line to check language
        lines = text.split("\n")
        fence = lines[start - 1] if start > 0 else ""
        if "ark" not in fence.lower() and not fence.strip().endswith("```"):
            # Only scan ark blocks or unmarked blocks
            if fence.strip() != "```" and "ark" not in fence.lower():
                continue
        for api in deprecated_apis:
            # Word-boundary match to avoid partial matches
            pattern = r'\b' + re.escape(api) + r'\b'
            if re.search(pattern, content):
                has_marker = has_deprecation_marker(content)
                # Also check 5 lines before the code block for an API note
                context_start = max(0, start - 6)
                context = "\n".join(lines[context_start:start])
                has_note = has_deprecation_marker(context) or "API note" in context or "API note" in content
                findings.append({
                    "file": str(path.relative_to(ROOT)),
                    "api": api,
                    "line": start,
                    "has_marker": has_marker or has_note,
                })
    return findings


def main() -> int:
    strict = "--strict" in sys.argv
    deprecated_apis = load_deprecated_apis()
    if not deprecated_apis:
        print("check-deprecated-api-in-docs: no deprecated APIs in manifest, skipping")
        return 0

    all_findings = []
    for path in SCAN_FILES:
        if not path.exists():
            continue
        findings = scan_file(path, deprecated_apis)
        all_findings.extend(findings)

    undocumented = [f for f in all_findings if not f["has_marker"]]

    if all_findings:
        print(f"check-deprecated-api-in-docs: {len(all_findings)} deprecated API occurrence(s) found")
        for f in all_findings:
            status = "documented" if f["has_marker"] else "UNDOCUMENTED"
            print(f"  [{status}] {f['file']}:{f['line']} — {f['api']}")

    if strict and undocumented:
        exempt_str = {str(p.relative_to(ROOT)) for p in STRICT_EXEMPT}
        strict_failures = [f for f in undocumented if f["file"] not in exempt_str]
        if strict_failures:
            print(f"\ncheck-deprecated-api-in-docs: FAIL ({len(strict_failures)} undocumented in strict-mode files)")
            for f in strict_failures:
                print(f"  ✗ {f['file']}:{f['line']} — {f['api']} lacks deprecation marker/note")
            return 1

    print("check-deprecated-api-in-docs: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
