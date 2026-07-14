#!/usr/bin/env python3
"""
Flatten emit_mir_inst_ctx dispatch from nested if/else to flat if/return.

Before:  if inst.op == 1 { handler } else {
         if inst.op == 2 { handler } else {
         ... }}}
After:   if inst.op == 1 { handler; return }
         if inst.op == 2 { handler; return }
         ...
"""

import re, shutil
from pathlib import Path

EMITTER_PATH = Path("src/compiler/emitter.ark")

def indent_of(line: str) -> int:
    return len(line) - len(line.lstrip())

def strip_line(line: str) -> str:
    return line.strip()

def is_op_if(line: str) -> bool:
    """Check if line is an `if inst.op == N {` statement."""
    return line.lstrip().startswith("if inst.op ==")

def find_closing_else(lines: list[str], start: int) -> int | None:
    """Find the `} else {` line at same indent as the if at lines[start]."""
    base_indent = indent_of(lines[start])
    for i in range(start + 1, len(lines)):
        if lines[i] is None:
            continue
        s = strip_line(lines[i])
        if "} else {" in s and indent_of(lines[i]) == base_indent:
            return i
        # If we hit a `}` at a LOWER indent than base, we've passed the matching close
        if s.startswith("}") and indent_of(lines[i]) < base_indent:
            return None
    return None

def main():
    with open(EMITTER_PATH) as f:
        lines = f.readlines()

    # Find emit_mir_inst_ctx function
    fn_start = None
    for i, line in enumerate(lines):
        if line.strip().startswith("fn emit_mir_inst_ctx"):
            fn_start = i
            break

    if fn_start is None:
        print("ERROR: Function not found")
        return 1

    fn_indent = indent_of(lines[fn_start])
    print(f"emit_mir_inst_ctx at line {fn_start}, indent={fn_indent}")

    # Find all `if inst.op == N` lines in the function
    op_lines = []
    for i in range(fn_start + 1, len(lines)):
        s = strip_line(lines[i])
        if is_op_if(s):
            indent = indent_of(lines[i])
            op_lines.append((i, indent, s))
        # Stop when function ends (dedent to fn level or beyond)
        if s == "}" and indent_of(lines[i]) <= fn_indent:
            break

    print(f"Found {len(op_lines)} opcode branches")
    if len(op_lines) == 0:
        print("ERROR: No opcode branches found")
        return 1

    # Build new content
    new_lines = list(lines)
    modifications = 0

    for idx, (op_idx, op_indent, op_content) in enumerate(op_lines):
        single_line = "}" in op_content[op_content.find("{"):]

        if single_line:
            # if inst.op == N { handler } else {
            new_content = op_content.replace("} else {", "; return }")
            new_lines[op_idx] = " " * op_indent + new_content + "\n"
            modifications += 1
        else:
            close_idx = find_closing_else(new_lines, op_idx)
            if close_idx is None:
                print(f"WARNING: No closing for multi-line branch at line {op_idx}")
                continue

            # Replace `} else {` with `} // end dispatch`
            close_indent = indent_of(new_lines[close_idx])
            new_lines[close_idx] = " " * close_indent + "}\n"
            # Add `return` before the closing brace
            new_lines.insert(close_idx, " " * (close_indent + 4) + "return\n")
            # Adjust subsequent close_idx references since we inserted a line
            for j in range(idx + 1, len(op_lines)):
                if op_lines[j][0] >= close_idx:
                    prev_idx = op_lines[j][0]
                    op_lines[j] = (prev_idx + 1, op_lines[j][1], op_lines[j][2])
            modifications += 1

    # After flattening, normalize indentation

    # First, find the emit_mir_inst_ctx function body range
    fn_body_indent = fn_indent + 4
    fn_body_end = len(new_lines) - 1
    for i in range(fn_start + 1, len(new_lines)):
        if new_lines[i] is None:
            continue
        s = strip_line(new_lines[i])
        if s == "}" and indent_of(new_lines[i]) <= fn_indent:
            fn_body_end = i
            break

    # Normalize indentation for all lines in the function body
    for i in range(fn_body_end + 1):
        if new_lines[i] is None:
            continue
        s = strip_line(new_lines[i])

        if s.lstrip().startswith("if inst.op =="):
            clean = s.lstrip()
            new_lines[i] = " " * fn_body_indent + clean + "\n"
        elif s.strip() == "return":
            new_lines[i] = " " * (fn_body_indent + 4) + "return\n"

    # Remove ALL standalone `}` lines within the function body that
    # are NOT immediately preceded by `return`
    # (These are chain braces from the old nested structure)
    for i in range(fn_start + 1, fn_body_end):
        if new_lines[i] is None:
            continue
        s = strip_line(new_lines[i])
        if s == "}":
            # Check if this is a multi-line handler closing (return before it)
            prev_idx = i - 1
            while prev_idx >= 0 and new_lines[prev_idx] is None:
                prev_idx -= 1
            is_handler_close = prev_idx >= 0 and strip_line(new_lines[prev_idx]).strip() == "return"

            if not is_handler_close:
                new_lines[i] = None

    # Filter out None lines
    new_lines = [l for l in new_lines if l is not None]
    for i in range(len(new_lines)):
        if new_lines[i] is None:
            continue
        s = strip_line(new_lines[i])

        if s.lstrip().startswith("if inst.op =="):
            clean = s.lstrip()
            new_lines[i] = " " * fn_body_indent + clean + "\n"
        elif s.strip() == "return":
            new_lines[i] = " " * (fn_body_indent + 4) + "return\n"
        elif s.strip() == "}":
            new_lines[i] = " " * fn_body_indent + "}\n"

    # Filter out None lines
    new_lines = [l for l in new_lines if l is not None]

    # Backup original
    shutil.copy2(EMITTER_PATH, EMITTER_PATH.with_suffix(".ark.bak2"))
    print(f"Backup: {EMITTER_PATH}.bak2")

    # Write
    with open(EMITTER_PATH, "w") as f:
        f.writelines(new_lines)

    print(f"Wrote {EMITTER_PATH}: {len(new_lines)} lines (was {len(lines)}), {modifications} modifications")
    return 0

if __name__ == "__main__":
    exit(main())
