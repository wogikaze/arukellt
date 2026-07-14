#!/usr/bin/env python3
"""
Extract intrinsic handler bodies from emitter.ark opcode 40 dispatch.

1. Identifies each callee branch (if eq(clone(callee), "name") { ... })
2. Creates emit_intrinsic.ark with one public function per branch
3. Replaces branch body with a function call in emitter.ark

Handles nested braces in handler bodies (while loops, if statements).
"""

import re, sys, shutil
from pathlib import Path

EMITTER_PATH = Path("src/compiler/emitter.ark")
INTRINSIC_PATH = Path("src/compiler/emit_intrinsic.ark")
DRY_RUN = "--dry-run" in sys.argv

def indent_of(line: str) -> int:
    return len(line) - len(line.lstrip())

def strip_line(line: str) -> str:
    return line.rstrip("\n").rstrip()

def callee_names(line: str) -> list[str]:
    return re.findall(r'eq\(clone\(callee\), "([^"]+)"\)', line)

def func_name(names: list[str]) -> str:
    preferred = [n for n in names if not n.startswith("__intrinsic")]
    if preferred:
        base = min(preferred, key=len)
    else:
        base = names[0]
    name = re.sub(r"[^a-zA-Z0-9_]", "_", base)
    name = re.sub(r"_+", "_", name).strip("_")
    return f"emit_{name}"

def next_callee_line(lines: list[str], start: int) -> int | None:
    """Find the next line starting with if eq(clone(callee ... from start."""
    for i in range(start, len(lines)):
        if lines[i].lstrip().startswith("if eq(clone(callee"):
            return i
    return None

def find_branch_end(lines: list[str], start: int) -> tuple[int, list[str], bool, bool]:
    """
    Given the line index of `if eq(clone(callee), ...) {`, find the closing
    `} else {` that ends this branch.

    Key insight: the if/else chain structure is:
        if eq(callee, "A") {       ← depth = 1 (branch A's own {)
            body
        } else {                   ← CLOSING: } closes A's block, else { opens next
        if eq(callee, "B") {
            body
        } else {
        ...
        }

    We detect branch end when we see `} else {` at the SAME indentation
    and with no nested unclosed braces (depth == 1 before the }).

    Returns: (end_line, body_lines, has_result_on_stack, is_single_line)
    """
    first = strip_line(lines[start])
    branch_indent = indent_of(lines[start])

    # Check if single-line: if ... { body }
    brace_pos = first.find("{")
    after_brace = first[brace_pos+1:].strip()
    if "}" in after_brace:
        body_text = after_brace[:after_brace.find("}")].strip()
        has_result = "result_on_stack" in body_text
        return (start, [body_text], has_result, True)

    # Multi-line: scan forward
    depth = 1  # The branch's own opening {
    body_lines = []
    has_result = False

    i = start + 1
    while i < len(lines):
        raw = strip_line(lines[i])

        if "result_on_stack" in raw:
            has_result = True

        # CRITICAL: Before counting braces, check if this line closes the branch
        # A `} else {` at the same indentation with depth == 1 closes the branch.
        # The `}` decreases depth (1 → 0), then `{` increases depth (0 → 1).
        # Net effect: depth stays at 1, but we detected the boundary.
        if depth == 1 and raw.lstrip().startswith("}"):
            # This could be the branch closing
            if "} else {" in raw or "} else{" in raw:
                return (i, body_lines, has_result, False)
            elif raw.lstrip().startswith("}") and "##" not in raw:
                # Bare } or } // comment — might be end of dispatch
                # Check that the } is at or above the branch indent level
                if indent_of(lines[i]) <= branch_indent:
                    return (i, body_lines, has_result, False)

        # Count braces in this line (ignoring string literals)
        opens = 0
        closes = 0
        in_str = False
        for ch in raw:
            if ch == '"':
                in_str = not in_str
            elif ch == '{' and not in_str:
                opens += 1
            elif ch == '}' and not in_str:
                closes += 1

        depth += opens - closes
        body_lines.append(raw)
        i += 1

    return (len(lines) - 1, body_lines, has_result, False)


# ---------------------------------------------------------------------------
# MAIN
# ---------------------------------------------------------------------------

def main():
    with open(EMITTER_PATH) as f:
        lines = f.readlines()

    # Find all callee branches
    branches = []
    i = 0
    while True:
        callee_line = next_callee_line(lines, i)
        if callee_line is None:
            break
        end_line, body, has_result, single = find_branch_end(lines, callee_line)
        names = callee_names(lines[callee_line])
        branches.append({
            "start": callee_line,
            "end": end_line,
            "names": names,
            "body": body,
            "has_result": has_result,
            "single": single,
        })
        i = end_line + 1

    print(f"Found {len(branches)} callee branches")
    # Show coverage
    first_line = min(b["start"] for b in branches)
    last_line = max(b["end"] for b in branches)
    print(f"Range: lines {first_line} to {last_line} (covers {last_line - first_line + 1} of {len(lines)} lines)")

    # Check for duplicate function names
    name_counts = {}
    for b in branches:
        fn = func_name(b["names"])
        name_counts[fn] = name_counts.get(fn, 0) + 1

    duplicates = {k: v for k, v in name_counts.items() if v > 1}
    if duplicates:
        print(f"WARNING: Duplicate function names: {duplicates}", file=sys.stderr)
        # Disambiguate by adding line number suffix
        for b in branches:
            fn = func_name(b["names"])
            if name_counts[fn] > 1:
                b["_func_name"] = f"{fn}_l{b['start']}"
            else:
                b["_func_name"] = fn
    else:
        for b in branches:
            b["_func_name"] = func_name(b["names"])

    # --- Generate emit_intrinsic.ark ---
    out = []
    out.append("// Arukellt Selfhost — Wasm Binary Emitter: Intrinsic Callee Handlers")
    out.append("// Auto-generated. Each handler replaces an inline branch in emitter.ark opcode 40 dispatch.")
    out.append("")
    out.append("use emit_opcodes")
    out.append("use emit_writer")
    out.append("use emit_scratch")
    out.append("use emit_inst_ctx")
    out.append("")

    for b in branches:
        fn = b["_func_name"]
        names_str = ", ".join(b["names"])
        out.append(f"// Handler for: {names_str}")
        out.append(f"pub fn {fn}(w: WasmWriter, inst: MirInst, ctx: emit_inst_ctx::SelfEmitCtx) -> bool {{")

        for body_line in b["body"]:
            if "result_on_stack" in body_line:
                continue  # handled by return value
            out.append(f"    {body_line}")

        # Determine return value
        if b["has_result"] and any("result_on_stack = false" in l for l in b["body"]):
            out.append("    false")
        else:
            out.append("    true")
        out.append("}")
        out.append("")

    if not DRY_RUN:
        with open(INTRINSIC_PATH, "w") as f:
            f.write("\n".join(out))

    print(f"Wrote {INTRINSIC_PATH} ({len(out)} lines, {len(branches)} functions)")

    # --- Generate modified emitter.ark ---
    new_lines = list(lines)
    replaced_count = 0
    removed_count = 0

    # Process in reverse so line numbers don't shift
    for b in reversed(branches):
        fn = b["_func_name"]
        start = b["start"]
        end = b["end"]
        branch_indent_val = indent_of(lines[start])
        indent_sp = " " * branch_indent_val

        # Build replacement for the branch
        first_line = strip_line(lines[start])
        brace_pos = first_line.find("{")
        condition = first_line[:brace_pos+1].strip()

        replaced_count += 1

        # For multi-line branches, replace body with function call
        if not b["single"] and end > start:
            new_first = f"{indent_sp}{condition}"
            new_body = f"{indent_sp}    result_on_stack = emit_intrinsic::{fn}(w, inst, ctx)"
            new_close = f"{indent_sp}}} else {{"
            new_lines[start] = new_first + "\n" + new_body + "\n" + new_close + "\n"

            # Mark intermediate lines for removal
            for j in range(start + 1, end + 1):
                if new_lines[j] is not None:
                    new_lines[j] = None
                    removed_count += 1
        else:
            # Single-line: if eq(...) { result_on_stack = emit_XXX(...) }
            new_lines[start] = f"{indent_sp}{condition}\n{indent_sp}    result_on_stack = emit_intrinsic::{fn}(w, inst, ctx)\n{indent_sp}}} else {{\n"

    print(f"Replaced {replaced_count} branches, marked {removed_count} lines for removal")

    # Filter out None lines
    new_lines = [l for l in new_lines if l is not None]

    # Write modified emitter.ark
    # First verify it looks reasonable
    output = "".join(new_lines)
    callee_count_after = output.count("if eq(clone(callee)")
    print(f"Modified emitter.ark: {len(new_lines)} lines (was {len(lines)}), {callee_count_after} callee if-lines remain")

    # Backup original (unless dry-run)
    if not DRY_RUN:
        shutil.copy2(EMITTER_PATH, EMITTER_PATH.with_suffix(".ark.bak"))
        print(f"Backup created: {EMITTER_PATH}.bak")

        with open(EMITTER_PATH, "w") as f:
            f.write(output)

        print(f"Wrote modified {EMITTER_PATH}")

        # Also update the use statement
        if "use emit_intrinsic\n" not in output and "use emit_intrinsic\r" not in output:
            if "use emit_intrinsic_io\n" in output:
                output = output.replace("use emit_intrinsic_io\n", "use emit_intrinsic\n")
            elif "use emit_inst_arith\n" in output:
                output = output.replace("use emit_inst_arith\n", "use emit_inst_arith\nuse emit_intrinsic\n")
                with open(EMITTER_PATH, "w") as f:
                    f.write(output)
                print("Added 'use emit_intrinsic' to emitter.ark")
    else:
        print(f"Modified emitter.ark: {len(new_lines)} lines (was {len(lines)}), {callee_count_after} callee if-refs remain")
        print(f"Estimated savings: ~{len(lines) - len(new_lines)} lines")

        # Check for structural issues
        if callee_count_after != 113:
            print(f"WARNING: Expected 113 callee references, found {callee_count_after}")

        # Show a sample of the output (first few replacements)
        sample_lines = [l for l in new_lines if "if eq(clone(callee" in l][:3]
        for sl in sample_lines[:1]:
            print(f"\nSample replacement (first if):\n{sl}")


if __name__ == "__main__":
    main()
