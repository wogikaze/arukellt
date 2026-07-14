#!/usr/bin/env python3
"""Advisory in-file test adoption report for issue #715 (always exits 0).

Reports test adoption broken down by:
- test module count (test mod "name" { ... })
- executable test case count (test "name" { ... })
- meaningful test case count (excludes trivial asserts)
- Phase 1 target modules (std/core, std/collections, std/text, std/bytes)
- Phase 2 target modules (lexer, parser, resolver, typechecker, mir, diagnostics)
- Out-of-scope modules (host, component, lsp, dap, wasm emitter body, simd)

The scanner ignores line comments, block comments, and string literals, so
commented-out examples or prose cannot inflate test counts.
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


def _strip_comments_and_strings(text: str) -> str:
    """Return text with line comments, block comments, and string literals
    replaced by spaces. This preserves character positions and line numbers."""
    out = []
    i = 0
    n = len(text)
    in_line_comment = False
    in_block_comment = False
    in_string = False
    while i < n:
        ch = text[i]
        prev = text[i - 1] if i > 0 else ""

        if in_line_comment:
            if ch == "\n":
                out.append(ch)
                in_line_comment = False
            else:
                out.append(" ")
            i += 1
            continue

        if in_block_comment:
            if ch == "*" and i + 1 < n and text[i + 1] == "/":
                out.append("  ")
                i += 2
                in_block_comment = False
                continue
            if ch == "\n":
                out.append(ch)
            else:
                out.append(" ")
            i += 1
            continue

        if in_string:
            if ch == '"' and prev != "\\":
                out.append(ch)
                in_string = False
            elif ch == "\n":
                out.append(ch)
            else:
                out.append(" ")
            i += 1
            continue

        if ch == '"':
            out.append(ch)
            in_string = True
            i += 1
            continue

        if ch == "/" and i + 1 < n:
            if text[i + 1] == "/":
                out.append("  ")
                i += 2
                in_line_comment = True
                continue
            if text[i + 1] == "*":
                out.append("  ")
                i += 2
                in_block_comment = True
                continue

        out.append(ch)
        i += 1

    return "".join(out)


def _read_identifier(text: str, i: int) -> tuple[str, int]:
    """Read an alphanumeric/underscore identifier starting at i."""
    n = len(text)
    start = i
    while i < n and (text[i].isalnum() or text[i] == "_"):
        i += 1
    return text[start:i], i


def _skip_whitespace(text: str, i: int) -> int:
    n = len(text)
    while i < n and text[i].isspace():
        i += 1
    return i


def _read_string_literal(text: str, i: int) -> tuple[str, int]:
    """Read a double-quoted string literal starting at i. Returns the contents
    (without quotes) and the index after the closing quote."""
    if i >= len(text) or text[i] != '"':
        return "", i
    i += 1
    n = len(text)
    start = i
    while i < n:
        ch = text[i]
        if ch == "\\" and i + 1 < n:
            i += 2
            continue
        if ch == '"':
            return text[start:i], i + 1
        i += 1
    return text[start:], i


def _extract_test_blocks(text: str) -> list[tuple[str, bool, str, int]]:
    """Extract test mod declarations and test cases from a source file.

    Returns a list of (name, is_mod, body_text, start_line). The scanner skips
    line comments, block comments, and string literals, so only actual test
    declarations are counted.
    """
    blocks: list[tuple[str, bool, str, int]] = []
    code = _strip_comments_and_strings(text)
    n = len(code)
    i = 0
    line_no = 1
    while i < n:
        ch = code[i]
        if ch == "\n":
            line_no += 1
        # Detect the keyword `test` as a bare word.
        if ch.isalpha() or ch == "_":
            ident, end = _read_identifier(code, i)
            if ident == "test":
                j = _skip_whitespace(code, end)
                is_mod = False
                if j < n and (code[j].isalpha() or code[j] == "_"):
                    next_ident, after = _read_identifier(code, j)
                    if next_ident == "mod":
                        is_mod = True
                        j = _skip_whitespace(code, after)
                if j < n and code[j] == '"':
                    name, after_name = _read_string_literal(code, j)
                    j = _skip_whitespace(code, after_name)
                    if j < n and code[j] == "{":
                        # Find the matching closing brace in the de-commented
                        # code. Strings are already replaced, so braces inside
                        # original strings are harmless here.
                        brace_depth = 1
                        k = j + 1
                        while k < n:
                            c = code[k]
                            if c == "{":
                                brace_depth += 1
                            elif c == "}":
                                brace_depth -= 1
                                if brace_depth == 0:
                                    break
                            k += 1
                        body = code[i : k + 1]
                        blocks.append((name, is_mod, body, line_no))
                        i = k + 1
                        continue
            i = end
            continue
        i += 1
    return blocks


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

            for test_name, is_mod, body, _ in _extract_test_blocks(text):
                if is_mod:
                    test_mod_count += 1
                else:
                    test_case_count += 1
                    if not is_trivial_test(test_name, body):
                        meaningful_count += 1

    return test_mod_count, test_case_count, meaningful_count


def main() -> int:
    print("in-file test adoption (advisory, #715)")
    print()

    invariant_failed = False

    # Phase 1: stdlib
    print("Phase 1: stdlib pure functions (target: 180 meaningful tests)")
    phase1_meaningful_total = 0
    for dir_rel, _ in sorted(PHASE1_DIRS.items()):
        mods, cases, meaningful = count_tests_in_dir(dir_rel)
        phase1_meaningful_total += meaningful
        if meaningful > cases:
            invariant_failed = True
            print(f"  ERROR {dir_rel}: meaningful_count ({meaningful}) > test_case_count ({cases})")
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
        if meaningful > cases:
            invariant_failed = True
            print(f"  ERROR {dir_rel}: meaningful_count ({meaningful}) > test_case_count ({cases})")
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

    if invariant_failed:
        print("\nADOPTION_INVARIANT_FAILED: meaningful_count > test_case_count")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
