#!/usr/bin/env python3
"""Advisory in-file test adoption report for issue #715 (always exits 0).

Reports test adoption broken down by:
- test module count (test mod "name" { ... })
- executable test case count (test "name" { ... })
- meaningful test case count (excludes trivial asserts)
- Phase 1 target modules (std/core, std/collections, std/text, std/bytes)
- Phase 2 target modules (lexer, parser, resolver, typechecker, mir, diagnostics)
- Out-of-scope modules (host, component, lsp, dap, wasm emitter body, simd)
"""
from __future__ import annotations

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Phase 1: stdlib pure functions
PHASE1_DIRS = {
    "std/core": 180,
    "std/collections": 0,
    "std/text": 0,
    "std/bytes": 0,
}
PHASE1_TOTAL_TARGET = 180

# Phase 2: compiler transformation passes
PHASE2_DIRS = {
    "src/compiler/lexer": 0,
    "src/compiler/parser": 0,
    "src/compiler/resolver": 0,
    "src/compiler/typechecker": 0,
    "src/compiler/mir": 0,
    "src/compiler/diagnostics": 0,
}
PHASE2_TOTAL_TARGET = 60

# Out-of-scope modules (side-effectful, fixture-only)
OUT_OF_SCOPE_DIRS = {
    "std/host",
    "std/component",
    "src/compiler/lsp",
    "src/compiler/dap",
    "src/compiler/analysis",
    "src/compiler/wasm",
    "src/compiler/simd",
}

# Trivial assert patterns (same as check-trivial-tests.py)
TRIVIAL_ASSERT_RES = [
    re.compile(r'assert\(\s*\d+\s*>=\s*0\s*\)'),
    re.compile(r'assert\(\s*true\s*\)'),
    re.compile(r'assert\(\s*1\s*==\s*1\s*\)'),
    re.compile(r'assert\(\s*0\s*==\s*0\s*\)'),
    re.compile(r'assert\(\s*false\s*==\s*false\s*\)'),
    re.compile(r'assert\(\s*\d+\s*==\s*\d+\s*\)'),
]
SELF_CMP_RE = re.compile(r'assert\(\s*(\w+)\s*==\s*\1\s*\)')
PROBE_NAME_RE = re.compile(r'test\s+"probe_\d+"')
SANITY_NAME_RE = re.compile(r'test\s+"sanity"')


def is_trivial_assert(line: str) -> bool:
    for pattern in TRIVIAL_ASSERT_RES:
        if pattern.search(line):
            return True
    if SELF_CMP_RE.search(line):
        return True
    return False


def is_trivial_test(name: str, body: str) -> bool:
    """Check if a test case is trivial (probe_N, sanity with trivial asserts, or only trivial asserts)."""
    if PROBE_NAME_RE.search(f'test "{name}"'):
        return True
    # Check if all asserts in body are trivial
    assert_lines = [l.strip() for l in body.splitlines() if "assert(" in l]
    if not assert_lines:
        return False
    return all(is_trivial_assert(l) for l in assert_lines)


def count_tests_in_dir(root_rel: str) -> tuple[int, int, int]:
    """Count test modules, test cases, and meaningful test cases in a directory.

    Returns (test_mod_count, test_case_count, meaningful_count).
    """
    root = os.path.join(ROOT, root_rel)
    if not os.path.isdir(root):
        return 0, 0, 0

    test_mod_count = 0
    test_case_count = 0
    meaningful_count = 0

    for dirpath, _, names in os.walk(root):
        for name in names:
            if not name.endswith(".ark"):
                continue
            path = os.path.join(dirpath, name)
            try:
                with open(path, encoding="utf-8") as f:
                    text = f.read()
            except Exception:
                continue

            # Count test mod declarations
            test_mod_count += len(re.findall(r'^\s*test\s+mod\s+"', text, re.M))

            # Count test cases (test "name" { ... }) — exclude test mod
            test_case_matches = re.findall(r'^\s*test\s+"([^"]+)"\s*\{', text, re.M)
            test_case_count += len(test_case_matches)

            # Count meaningful test cases (exclude trivial)
            # Extract test blocks
            lines = text.splitlines()
            i = 0
            while i < len(lines):
                line = lines[i]
                m = re.search(r'test\s+"([^"]+)"\s*\{', line)
                if m and "test mod" not in line:
                    test_name = m.group(1)
                    # Find closing brace
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
                    if not is_trivial_test(test_name, body_text):
                        meaningful_count += 1
                    i = j + 1
                else:
                    i += 1

    return test_mod_count, test_case_count, meaningful_count


def main() -> int:
    print("in-file test adoption (advisory, #715)")
    print()

    # Phase 1: stdlib
    print("Phase 1: stdlib pure functions (target: 180 meaningful tests)")
    phase1_meaningful_total = 0
    for dir_rel, _ in sorted(PHASE1_DIRS.items()):
        mods, cases, meaningful = count_tests_in_dir(dir_rel)
        phase1_meaningful_total += meaningful
        print(f"  {dir_rel}: {mods} test mods, {cases} test cases, {meaningful} meaningful")
    p1_status = "ok" if phase1_meaningful_total >= PHASE1_TOTAL_TARGET else "below-target"
    print(f"  Phase 1 total: {phase1_meaningful_total}/{PHASE1_TOTAL_TARGET} meaningful [{p1_status}]")
    print()

    # Phase 2: compiler
    print("Phase 2: compiler transformation passes (target: 60 meaningful tests)")
    phase2_meaningful_total = 0
    for dir_rel, _ in sorted(PHASE2_DIRS.items()):
        mods, cases, meaningful = count_tests_in_dir(dir_rel)
        phase2_meaningful_total += meaningful
        print(f"  {dir_rel}: {mods} test mods, {cases} test cases, {meaningful} meaningful")
    p2_status = "ok" if phase2_meaningful_total >= PHASE2_TOTAL_TARGET else "below-target"
    print(f"  Phase 2 total: {phase2_meaningful_total}/{PHASE2_TOTAL_TARGET} meaningful [{p2_status}]")
    print()

    # Out-of-scope
    print("Out-of-scope modules (should have 0 in-file tests):")
    for dir_rel in sorted(OUT_OF_SCOPE_DIRS):
        mods, cases, meaningful = count_tests_in_dir(dir_rel)
        if cases > 0:
            print(f"  WARNING: {dir_rel}: {cases} test cases (should be fixture-only)")
        else:
            print(f"  {dir_rel}: 0 (ok)")

    return 0


if __name__ == "__main__":
    sys.exit(main())
