#!/usr/bin/env python3
"""Split src/compiler/mir.ark into logical module files with prefix rewriting.

Usage: python3 scripts/util/split_mir.py [--dry-run]

Sections:
  - mir_opcodes.ark (1-224):  54 opcode constant functions
  - mir_ir.ark (225-1126):    65 type/SSA definitions + constructors
  - mir_type_info.ark (1127-1415): 65 type system constants
  - mir_lower.ark (1416-5282):  41 lowering functions
  - mir_dump.ark (5283-6417):   24 dump + entry + tag functions
"""

import re
import sys
from pathlib import Path

SRC = Path("src/compiler/mir.ark")

# (section_name, start_0idx, end_exclusive, module_name, use_imports)
SECTIONS = [
    ("opcodes",  0,    224,  "mir_opcodes", ["use std::host::env", "use std::host::process", "use std::host::stdio"]),
    ("ir_types", 224,  1126, "mir_ir",      ["use std::host::stdio"]),
    ("type_info", 1126, 1414, "mir_type_info", []),
    ("lowering", 1414, 5282, "mir_lower",   ["use std::host::env", "use std::host::stdio"]),
    ("dump",     5282, 6417, "mir_dump",    ["use std::host::process", "use std::host::stdio"]),
]

def function_names_in_section(lines):
    """Extract top-level function names from a list of lines."""
    fns = set()
    for line in lines:
        stripped = line.strip()
        m = re.match(r'^(?:pub\s+)?fn\s+(\w+)\(', stripped)
        if m:
            fns.add(m.group(1))
    return fns


def find_calls(text: str, known_fns: set[str]) -> set[str]:
    """Find function calls to known_fns that are NOT already module-prefixed."""
    calls = set()
    for m in re.finditer(r'(?<![:\w.])([a-zA-Z_]\w+)\s*\(', text):
        name = m.group(1)
        if name in known_fns:
            calls.add(name)
    return calls


def rewrite_calls(text: str, call_map: dict[str, str]) -> str:
    """Rewrite bare function calls with module prefix.

    call_map: {func_name: "module_prefix"}

    Handles edge cases: don't rewrite after '::' or 'fn ' or '.' or in comments.
    """
    # Sort by length descending to match longer names first
    items = sorted(call_map.items(), key=lambda x: -len(x[0]))

    result = text
    for func_name, module_name in items:
        # Match func_name( but not ::func_name( or fn func_name( or .func_name(
        pattern = r'(?<![:\w.])\b' + re.escape(func_name) + r'\s*\('
        replacement = f'{module_name}::{func_name}('
        result = re.sub(pattern, replacement, result)
    return result


def write_section_file(sname: str, mname: str, text: str, use_imports: list[str], needed_modules: set[str], is_dry_run: bool, call_map: dict[str, str] = None):
    """Build and write a section file with header, use imports, and content."""
    new_lines = text.split('\n')
    out_lines = []

    # Header
    title = sname.replace('_', ' ').title()
    out_lines.append(f"// Arukellt Selfhost — MIR {title}")
    out_lines.append("//")
    out_lines.append("// Extracted from mir.ark for module-level organization.")
    out_lines.append("")

    # Use imports (std::host)
    existing_uses = set()
    for l in new_lines:
        l = l.strip()
        if l.startswith("use "):
            existing_uses.add(l)
    for imp in use_imports:
        if imp not in existing_uses:
            out_lines.append(imp)

    # Module use imports
    existing_module_uses = set()
    for l in new_lines:
        l = l.strip()
        if l.startswith("use ") and not l.startswith("use std"):
            existing_module_uses.add(l.split()[1] if len(l.split()) > 1 else "")
    for mod in sorted(needed_modules):
        if mod != mname and f"use {mod}" not in existing_module_uses:
            out_lines.append(f"use {mod}")

    # Blank line after uses
    if len(out_lines) > 5:
        out_lines.append("")

    # Add content, skipping original use std:: lines near the top
    for i, l in enumerate(new_lines):
        stripped = l.strip()
        if stripped.startswith("use std::") and i < 10:
            continue  # Already in header
        if stripped.startswith("use ") and not stripped.startswith("use std") and i < 10:
            continue  # Module use lines already added
        out_lines.append(l)

    if is_dry_run:
        print(f"  Would write {mname}.ark ({len(out_lines)} lines)")
        return

    write_path = Path("src/compiler") / f"{mname}.ark"
    write_path.write_text('\n'.join(out_lines))
    print(f"  Wrote {write_path.name} ({len(out_lines)} lines)")


def main():
    dry_run = "--dry-run" in sys.argv

    lines = SRC.read_text().splitlines(keepends=False)
    total_lines = len(lines)
    print(f"mir.ark: {total_lines} lines total")

    # Separate lines into sections
    sections_data = {}
    for sname, start, end, _, _ in SECTIONS:
        sections_data[sname] = lines[start:end]

    # Get function names defined in each section
    section_fns = {}
    for sname, _, _, mname, _ in SECTIONS:
        section_fns[sname] = function_names_in_section(sections_data[sname])
        print(f"  {sname} ({mname}): {len(section_fns[sname])} functions")

    # Build global function→module mapping
    func_to_module = {}
    for sname, _, _, mname, _ in SECTIONS:
        for fn in section_fns[sname]:
            func_to_module[fn] = mname

    # For each section, identify calls to other-section functions
    all_fns = set(func_to_module.keys())

    for sname, _, _, mname, use_imports in SECTIONS:
        text = '\n'.join(sections_data[sname])
        my_fns = section_fns[sname]
        other_fns = all_fns - my_fns
        calls = find_calls(text, other_fns)

        # Build call map
        call_map = {}
        for fn in calls:
            call_map[fn] = func_to_module[fn]

        # Determine needed module imports
        needed_modules = set(call_map.values())
        needed_modules.discard(mname)

        if calls and not dry_run:
            # Rewrite calls
            rewritten = rewrite_calls(text, call_map)
        else:
            rewritten = text

        if dry_run:
            if calls:
                print(f"  {mname}: {len(calls)} cross-module calls to rewrite")
            else:
                print(f"  {mname}: no cross-module calls → clean")
            continue

        write_section_file(sname, mname, rewritten, use_imports, needed_modules, False, call_map)

    if dry_run:
        print(f"\nDry-run complete. Run without --dry-run to write files.")


if __name__ == "__main__":
    main()
