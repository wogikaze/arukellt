#!/usr/bin/env python3
"""
Lightweight catalog of intrinsic callee branches in emitter.ark.
Outputs: start_line | callee_names | body_line_count | has_result_on_stack
"""

import re, sys
from pathlib import Path

EMITTER_PATH = Path("src/compiler/emitter.ark")

def strip_indent(line: str) -> tuple[int, str]:
    raw = line.rstrip("\n").rstrip()
    stripped = raw.lstrip()
    return len(raw) - len(stripped), stripped

with open(EMITTER_PATH) as f:
    lines = f.readlines()

# Find start of opcode 40
op40_line = None
for i, line in enumerate(lines):
    if 'if inst.op == 40' in line and '{' in line:
        op40_line = i
        break
print(f"Opcode 40 starts at line {op40_line}")

# Find end of opcode 40
# It closes when we see '} else {' at indentation level 0 of the opcode 40 block
op40_indent, _ = strip_indent(lines[op40_line])
# The closing } else { should be at this indent level
closing_line = None
for i in range(op40_line + 1, len(lines)):
    indent, content = strip_indent(lines[i])
    if indent == op40_indent and content.rstrip().startswith('}') and ' else {' in content:
        closing_line = i
        break
print(f"Opcode 40 closing at line {closing_line}")

# Now scan for callee if-lines between op40_line and closing_line
# Track brace depth relative to the outer if-inst-op-40
branches = []
i = op40_line + 1

# The intrinsic dispatch is inside a `if inst.op == 40 { ... } else {`
# Inside that, the callee chain is:
#   let mut result_on_stack = true
#   if eq(callee, "name1") { body } else {
#   if eq(callee, "name2") { body } else {
#   ...
#   let fn_idx = resolve_fn_index(...)  -- fallthrough
#   }
#   } -- end of opcode 40

current = None
depth = 0
in_intrinsic_block = False

while i < len(lines):
    indent, content = strip_indent(lines[i])

    if in_intrinsic_block:
        # Track brace depth
        if content.rstrip().endswith('{') or content.rstrip().endswith('{ '):
            depth += 1
        if content.startswith('}'):
            depth -= 1
            if depth == 0:
                # End of intrinsic block (or outer)
                in_intrinsic_block = False
                i += 1
                continue

        # Look for callee branch start
        if content.startswith('if eq(clone(callee)'):
            names = re.findall(r'eq\(clone\(callee\), "([^"]+)"\)', content)
            if names:
                current = {
                    'line': i,
                    'names': names,
                    'single_line': '}' in content[content.find('{')+1:],
                    'has_result': False,
                    'last_body_line': i,
                }
                branches.append(current)
                if current['single_line']:
                    current['has_result'] = 'result_on_stack' in content
        elif current and not current['single_line']:
            if 'result_on_stack' in content:
                current['has_result'] = True
            current['last_body_line'] = i

        # Detect the fallthrough (resolve_fn_index)
        if 'resolve_fn_index' in content:
            in_intrinsic_block = False

    elif content.startswith('if eq(clone(callee)'):
        in_intrinsic_block = True
        names = re.findall(r'eq\(clone\(callee\), "([^"]+)"\)', content)
        if names:
            current = {
                'line': i,
                'names': names,
                'single_line': '}' in content[content.find('{')+1:],
                'has_result': False,
                'last_body_line': i,
            }
            branches.append(current)
            if current['single_line']:
                current['has_result'] = 'result_on_stack' in content
        depth = 0
        brace_idx = content.find('{')
        if brace_idx >= 0:
            depth = content[brace_idx:].count('{') - content[brace_idx:].count('}')
    elif 'resolve_fn_index' in content:
        break

    i += 1

print(f"\nFound {len(branches)} callee branches (+1 fallthrough)")
print(f"{'Line':>6}  {'Names':<30} {'Lines':>5}  {'ROS':>3}  {'Type':>6}")
print("-" * 70)
for b in branches:
    line = b['line']
    names = ", ".join(b['names'][:2])
    if len(b['names']) > 2:
        names += f" (+{len(b['names'])-2})"
    body_lines = b['last_body_line'] - b['line']
    ros = "Y" if b['has_result'] else "N"
    stype = "single" if b['single_line'] else "multi"
    print(f"{line:>6}  {names:<30} {body_lines:>5}  {ros:>3}  {stype:>6}")
