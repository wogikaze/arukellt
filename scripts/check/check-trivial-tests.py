#!/usr/bin/env python3
"""Detect trivial/dummy in-file tests that inflate test counts without
verifying any contract, boundary, or invariant.

Detects:
- test name "probe_N" (numbered probe tests)
- test name "sanity" with body containing only trivial asserts
- assert(literal >= 0) where literal is an integer constant
- assert(x == x) equivalent (self-comparison)
- assert(true), assert(1 == 1), assert(0 == 0), assert(false == false)

Parses test blocks (test "name" { ... }) and checks all assert statements
within each block, including multi-line blocks.

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

# Trivial assert patterns — these match any assert that is always true
TRIVIAL_ASSERT_RES = [
    re.compile(r'assert\(\s*\d+\s*>=\s*0\s*\)'),          # assert(N >= 0) for literal N
    re.compile(r'assert\(\s*true\s*\)'),                    # assert(true)
    re.compile(r'assert\(\s*true\s*==\s*true\s*\)'),      # assert(true == true)
    re.compile(r'assert\(\s*false\s*==\s*false\s*\)'),    # assert(false == false)
    re.compile(r'assert\(\s*(\d+)\s*==\s*\1\s*\)'),      # assert(N == N) for same literal N
]

# Self-comparison pattern: assert(x == x) where both sides are the same identifier
SELF_CMP_RE = re.compile(r'assert\(\s*(\w+)\s*==\s*\1\s*\)')

# Test block pattern: test "name" { ... }
TEST_BLOCK_START_RE = re.compile(r'test\s+"([^"]+)"\s*\{')

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


def extract_test_blocks(text: str) -> list[tuple[str, int, str]]:
    """Extract test blocks from text.

    Returns list of (test_name, start_line, body_text).
    Handles both single-line and multi-line test blocks.
    """
    blocks = []
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        m = TEST_BLOCK_START_RE.search(line)
        if m:
            test_name = m.group(1)
            start_line = i + 1
            # Find the closing brace
            body_lines = []
            brace_depth = line.count("{") - line.count("}")
            j = i
            while j < len(lines):
                if j > i:
                    brace_depth += lines[j].count("{") - lines[j].count("}")
                body_lines.append(lines[j])
                if brace_depth <= 0:
                    break
                j += 1
            body_text = "\n".join(body_lines)
            blocks.append((test_name, start_line, body_text))
            i = j + 1
        else:
            i += 1
    return blocks


def is_trivial_assert(line: str) -> bool:
    """Check if a line contains a trivial assert."""
    for pattern in TRIVIAL_ASSERT_RES:
        if pattern.search(line):
            return True
    # Check for self-comparison: assert(x == x)
    if SELF_CMP_RE.search(line):
        return True
    return False


def find_trivial_tests(path: Path) -> list[str]:
    """Return list of findings for trivial tests in the given file."""
    findings = []
    text = path.read_text(encoding="utf-8")

    blocks = extract_test_blocks(text)

    for test_name, start_line, body_text in blocks:
        # Check for probe_N test names
        if PROBE_NAME_RE.search(f'test "{test_name}"'):
            findings.append(f"{path}:{start_line}: TRIVIAL_TEST: probe_N pattern: test \"{test_name}\"")
            continue

        # Check for sanity test name
        is_sanity = SANITY_NAME_RE.search(f'test "{test_name}"') is not None

        # Check all lines in the body for trivial asserts
        body_lines = body_text.splitlines()
        trivial_count = 0
        total_asserts = 0
        for body_line in body_lines:
            stripped = body_line.strip()
            if "assert(" not in stripped:
                continue
            total_asserts += 1
            if is_trivial_assert(stripped):
                trivial_count += 1
                findings.append(f"{path}:{start_line}: TRIVIAL_ASSERT in test \"{test_name}\": {stripped}")

        # If sanity test and all asserts are trivial, flag it
        if is_sanity and total_asserts > 0 and trivial_count == total_asserts:
            if not any("TRIVIAL_TEST: sanity" in f for f in findings if f"{path}:{start_line}:" in f):
                findings.append(f"{path}:{start_line}: TRIVIAL_TEST: sanity with only trivial asserts")

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
