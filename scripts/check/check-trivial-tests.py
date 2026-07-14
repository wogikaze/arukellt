#!/usr/bin/env python3
"""Detect trivial/dummy in-file tests that inflate test counts without
verifying any contract, boundary, or invariant.

Detects:
- test name "probe_N" (numbered probe tests)
- test name "sanity" with body containing only trivial asserts
- assert(literal >= 0) where literal is an integer constant
- assert(x == x) equivalent (self-comparison)
- assert(true), assert(1 == 1), assert(0 == 0), assert(false == false)

Usage:
    check-trivial-tests.py [--root <path>]

Exit 0 if no trivial tests found, exit 1 if any found.
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

# Patterns that indicate trivial/dummy tests
PROBE_NAME_RE = re.compile(r'test\s+"probe_\d+"')
SANITY_NAME_RE = re.compile(r'test\s+"sanity"')
TRIVIAL_ASSERT_RES = [
    re.compile(r'assert\(\s*\d+\s*>=\s*0\s*\)'),
    re.compile(r'assert\(\s*true\s*\)'),
    re.compile(r'assert\(\s*1\s*==\s*1\s*\)'),
    re.compile(r'assert\(\s*0\s*==\s*0\s*\)'),
    re.compile(r'assert\(\s*false\s*==\s*false\s*\)'),
    re.compile(r'assert\(\s*\d+\s*==\s*\d+\s*\)'),
]

# Directories to scan
SCAN_DIRS = [
    REPO_ROOT / "src" / "compiler",
    REPO_ROOT / "std",
]

# Files to exclude (fixtures that test assert itself)
EXCLUDE_FILES = {
    "tests/fixtures/test_assert/assert_pass.ark",
    "tests/fixtures/test_syntax/basic.ark",
}


def find_trivial_tests(path: Path) -> list[str]:
    """Return list of findings for trivial tests in the given file."""
    findings = []
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()

    for i, line in enumerate(lines, 1):
        stripped = line.strip()

        # Check for probe_N test names
        if PROBE_NAME_RE.search(stripped):
            findings.append(f"{path}:{i}: TRIVIAL_TEST: probe_N pattern: {stripped}")
            continue

        # Check for sanity test with trivial body
        if SANITY_NAME_RE.search(stripped):
            # Look at the next few lines for the body
            body = []
            for j in range(i, min(i + 5, len(lines))):
                body.append(lines[j].strip())
                if "}" in lines[j]:
                    break
            body_text = " ".join(body)
            # Check if body contains only trivial asserts
            has_real_assert = False
            for pattern in TRIVIAL_ASSERT_RES:
                if pattern.search(body_text):
                    has_real_assert = True
                    break
            # If the body is just assert(1 == 1) or similar
            if has_real_assert or "assert(1 + 1 == 2)" not in body_text:
                # Check specifically for trivial patterns
                is_trivial = False
                for pattern in TRIVIAL_ASSERT_RES:
                    if pattern.search(body_text):
                        is_trivial = True
                        break
                if is_trivial:
                    findings.append(f"{path}:{i}: TRIVIAL_TEST: sanity with trivial assert: {stripped}")

        # Check for trivial assert patterns on any test line
        for pattern in TRIVIAL_ASSERT_RES:
            if pattern.search(stripped):
                # Only flag if inside a test block (heuristic: line is indented)
                if stripped.startswith("test ") or "test " in stripped:
                    findings.append(f"{path}:{i}: TRIVIAL_ASSERT: {stripped}")
                    break

    return findings


def main() -> int:
    parser = argparse.ArgumentParser(description="Detect trivial/dummy tests")
    parser.add_argument("--root", default=str(REPO_ROOT))
    args = parser.parse_args()
    root = Path(args.root)

    all_findings = []
    for scan_dir in SCAN_DIRS:
        if not scan_dir.exists():
            continue
        for p in scan_dir.rglob("*.ark"):
            rel = p.relative_to(root)
            if str(rel) in EXCLUDE_FILES:
                continue
            all_findings.extend(find_trivial_tests(p))

    if all_findings:
        for f in all_findings:
            print(f)
        print(f"\nTRIVIAL_TESTS: {len(all_findings)} finding(s)")
        return 1
    else:
        print("TRIVIAL_TESTS: PASS")
        return 0


if __name__ == "__main__":
    sys.exit(main())
