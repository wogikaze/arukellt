#!/usr/bin/env python3
"""Split src/compiler/parser.ark into parser_kinds.ark + parser.ark.

Extracts NK_*, OP_*, UOP_*, and type-related constant functions into
parser_kinds.ark, rewrites calls in the remaining parser.ark to use
parser_kinds:: prefix.

Usage: python3 scripts/util/split_parser.py [--dry-run]
"""

import re
import sys
from pathlib import Path

SRC = Path("src/compiler/parser.ark")


def find_defined_functions(text: str) -> dict[str, int]:
    """Return {func_name: line_index} for all top-level fn definitions."""
    fns = {}
    for i, line in enumerate(text.split('\n')):
        m = re.match(r'^fn\s+(\w+)\s*\(', line.strip())
        if m:
            fns[m.group(1)] = i
    return fns


def is_const_kind_func(name: str) -> bool:
    """Return True if this function is a NK_/OP_/UOP_ constant function."""
    return name.startswith('NK_') or name.startswith('OP_') or name.startswith('UOP_')


def find_calls(text: str, known_fns: set[str]) -> set[str]:
    """Find bare calls to known_fns (not already module-prefixed)."""
    calls = set()
    for m in re.finditer(r'(?<![:\w.])([A-Z_]\w*)\s*\(', text):
        name = m.group(1)
        if name in known_fns:
            calls.add(name)
    return calls


def rewrite_calls(text: str, call_map: dict[str, str]) -> str:
    """Rewrite bare function calls with module prefix."""
    items = sorted(call_map.items(), key=lambda x: -len(x[0]))
    result = text
    for func_name, module_name in items:
        pattern = r'(?<![:\w.])\b' + re.escape(func_name) + r'\s*\('
        replacement = f'{module_name}::{func_name}('
        result = re.sub(pattern, replacement, result)
    return result


def main():
    dry_run = "--dry-run" in sys.argv

    text = SRC.read_text()
    lines = text.split('\n')
    total_lines = len(lines)
    print(f"parser.ark: {total_lines} lines total")

    # Find all defined functions and classify them
    all_fns = find_defined_functions(text)
    const_fns = {name: idx for name, idx in all_fns.items() if is_const_kind_func(name)}
    other_fns = {name: idx for name, idx in all_fns.items() if not is_const_kind_func(name)}

    print(f"  Const functions (NK_/OP_/UOP_): {len(const_fns)}")
    print(f"  Other functions: {len(other_fns)}")

    # Determine the line range for constants section
    # Lines 1-8 are header/imports, lines 10-318 define the constants
    const_start_line = 9  # 0-indexed, line 10 is first const fn
    # Find the last const function definition
    const_end_line = max(const_fns.values())
    # The section ends after UOP_BITNOT at line 318 (0-indexed: 317)
    # But let's be safe - the section goes to where the Span struct starts
    # Line 319 (0-indexed: 318) starts "// ── Span ──"
    # Let's find the actual end: look for the line after UOP_BITNOT's closing brace
    const_section_end = 318  # 0-indexed, exclusive (lines 0-318 inclusive = 319 lines)

    # Split the text: constants section vs rest
    # Lines 0-8: header/imports
    header_lines = lines[:9]  # Lines 0-8 (9 lines): comments + uses
    
    # Lines 9-318 (0-indexed): constants (310 lines, 10-318 1-indexed)
    const_lines = lines[9:const_section_end + 1]
    
    # Lines 319-end: struct defs + parsing functions + AST comments
    body_lines = lines[const_section_end + 1:]

    # Remove trailing empty lines from const section
    while const_lines and const_lines[-1].strip() == '':
        const_lines.pop()

    const_text = '\n'.join(const_lines)
    body_text = '\n'.join(body_lines)

    # Find calls from body into const fns
    const_fn_names = set(const_fns.keys())
    calls = find_calls(body_text, const_fn_names)

    print(f"\n  Const functions called from body: {len(calls)}")
    uncalled = const_fn_names - calls
    if uncalled:
        print(f"  Const functions NOT called from body: {', '.join(sorted(uncalled))}")

    # Build parser_kinds.ark content
    parser_kinds_lines = [
        "// Arukellt Selfhost — Parser Node Kind Constants",
        "//",
        "// Extracted from parser.ark for module-level organization.",
        "// Contains NK_*, OP_*, UOP_* node kind and operator constant functions.",
        "",
    ]

    # Add the const functions (skip original imports/comments at top)
    # Skip lines that are the original header/imports
    skip_prefixes = [
        "// Arukellt Parser",
        "//",
        "// Consumes a Vec",
        "// Uses i32 tag",
        "use std::host::stdio",
        "use lexer",
        "use diagnostics",
    ]

    for line in const_lines:
        stripped = line.strip()
        skip = False
        for prefix in skip_prefixes:
            if stripped.startswith(prefix):
                skip = True
                break
        if skip:
            continue
        parser_kinds_lines.append(line)

    # Add the AST Node Kind Tags comment section from body
    # Find the "// ── AST Node Kind Tags" section
    ast_tags_start = None
    for i, line in enumerate(body_lines):
        if "AST Node Kind Tags" in line:
            ast_tags_start = i
            break
    
    if ast_tags_start is not None:
        # The AST tags section is comments only
        parser_kinds_lines.append("")
        for line in body_lines[ast_tags_start:]:
            parser_kinds_lines.append(line)

    parser_kinds_text = '\n'.join(parser_kinds_lines)
    # Remove trailing empty lines
    parser_kinds_text = parser_kinds_text.rstrip('\n') + '\n'

    # Build the new parser.ark content
    # Start with header + imports
    new_parser_lines = list(header_lines)
    # Add use parser_kinds after the existing uses
    new_parser_lines.append("use parser_kinds")
    new_parser_lines.append("")

    # Add the body (struct defs through end), but skip the AST Node Kind Tags comments
    body_text_without_tags = ''
    if ast_tags_start is not None:
        body_lines_before_tags = body_lines[:ast_tags_start]
        body_text_without_tags = '\n'.join(body_lines_before_tags)
    else:
        body_text_without_tags = body_text

    # Rewrite calls in the body text
    call_map = {fn: "parser_kinds" for fn in calls}
    rewritten_body = rewrite_calls(body_text_without_tags, call_map)

    new_parser_lines.append(rewritten_body)

    new_parser_text = '\n'.join(new_parser_lines)
    new_parser_text = new_parser_text.rstrip('\n') + '\n'

    if dry_run:
        print(f"\n  Would write parser_kinds.ark ({len(parser_kinds_lines)} lines)")
        print(f"  Would rewrite parser.ark ({new_parser_text.count(chr(10)) + 1} lines)")
        print(f"  Would add 'use parser_kinds' to parser.ark")
        print(f"  Would rewrite {len(calls)} function calls with parser_kinds:: prefix")
        print(f"\nDry-run complete. Run without --dry-run to write files.")
        return

    # Write parser_kinds.ark
    write_path = Path("src/compiler") / "parser_kinds.ark"
    write_path.write_text(parser_kinds_text)
    print(f"\n  Wrote {write_path.name} ({len(parser_kinds_lines)} lines)")

    # Write new parser.ark
    SRC.write_text(new_parser_text)
    new_line_count = new_parser_text.count('\n')
    print(f"  Rewrote parser.ark ({new_line_count} lines)")
    print(f"  Added 'use parser_kinds' to parser.ark")
    print(f"  Rewrote {len(calls)} function calls with parser_kinds:: prefix")


if __name__ == "__main__":
    main()
