#!/usr/bin/env python3
"""Merge Arukellt compiler source files into a single file."""

base = "/home/wogikaze/arukellt.worktrees/feature-arukellt-v1/src/compiler"

def read(fname):
    with open(f"{base}/{fname}") as f:
        return f.read()

def before_fn_main(text):
    """Get content before fn main(), also removing the entry point section header."""
    idx = text.find('\nfn main()')
    if idx == -1:
        return text
    # Also strip the Entry Point section comment if present
    ep_marker = '\n// ── Entry Point'
    ep_idx = text.rfind(ep_marker, 0, idx)
    if ep_idx != -1:
        return text[:ep_idx]
    return text[:idx]

def remove_between(text, start_marker, end_marker):
    """Remove text from start_marker (inclusive) to end_marker (exclusive)."""
    start_idx = text.find(start_marker)
    if start_idx == -1:
        print(f"WARNING: start_marker not found: {start_marker[:60]!r}")
        return text
    end_idx = text.find(end_marker, start_idx)
    if end_idx == -1:
        print(f"WARNING: end_marker not found after start: {end_marker[:60]!r}")
        return text[:start_idx]
    return text[:start_idx] + text[end_idx:]

def remove_fn(text, fn_signature):
    """Remove a function starting with fn_signature up to the closing brace."""
    idx = text.find('\n' + fn_signature)
    if idx == -1:
        print(f"WARNING: function not found: {fn_signature!r}")
        return text
    # Find the end of the function by counting braces
    start = idx + 1  # skip the \n
    brace_start = text.find('{', start)
    if brace_start == -1:
        return text
    depth = 0
    i = brace_start
    while i < len(text):
        c = text[i]
        if c == '{':
            depth += 1
        elif c == '}':
            depth -= 1
            if depth == 0:
                # Remove from the \n before fn_signature to after the closing }
                end = i + 1
                # Skip trailing newline
                if end < len(text) and text[end] == '\n':
                    end += 1
                return text[:idx] + text[end:]
        i += 1
    return text

# ──────────────────────────────────────────────────────────────────────────────
# Header
# ──────────────────────────────────────────────────────────────────────────────
header = """// Arukellt Self-Hosted Compiler — merged single-file build
// Generated from: lexer.ark, parser.ark, resolver.ark, typechecker.ark,
//                 mir.ark, emitter.ark, driver.ark, main.ark

use std::host::stdio
use std::host::fs
use std::host::env
"""

# ──────────────────────────────────────────────────────────────────────────────
# New main()
# ──────────────────────────────────────────────────────────────────────────────
new_main = """fn main() {
    let argc = env::arg_count()
    if argc == 0 {
        stdio::eprintln(String_from("usage: arukellt <input.ark>"))
        return
    }
    let input_path = env::arg_at(0)
    let result = fs::read_to_string(clone(input_path))
    match result {
        Ok(source) => {
            let tokens = tokenize(source)
            let tok_count = len(tokens)
            let p = Parser_new(tokens)
            let decls = parse_module(p)
            let err_count = len(p.errors)
            if err_count > 0 {
                let mut i = 0
                while i < err_count {
                    let e = get_unchecked(p.errors, i)
                    stdio::eprintln(concat(String_from("error: "), clone(e.message)))
                    i = i + 1
                }
            } else {
                stdio::println(concat(String_from("tokens: "), i32_to_string(tok_count)))
                stdio::println(concat(String_from("declarations: "), i32_to_string(len(decls))))
            }
        },
        Err(e) => {
            stdio::eprintln(concat(String_from("cannot read file: "), e))
        },
    }
}
"""

parts = []

# ──────────────────────────────────────────────────────────────────────────────
# 1. lexer.ark
# ──────────────────────────────────────────────────────────────────────────────
lex = read("lexer.ark")
lex = lex.replace("use std::host::stdio\n", "", 1)
lex = before_fn_main(lex)
parts.append(lex.rstrip())

# ──────────────────────────────────────────────────────────────────────────────
# 2. parser.ark
# ──────────────────────────────────────────────────────────────────────────────
par = read("parser.ark")
par = par.replace("use std::host::stdio\n", "", 1)

# Remove TK_* re-exports section (from its header to the Token duplicate section header)
par = remove_between(par,
    "\n// ── Token Kind Re-exports (from lexer)",
    "\n// ── Token (duplicated from lexer)")

# Remove Token duplicate section (from its header to Parser State section)
par = remove_between(par,
    "\n// ── Token (duplicated from lexer)",
    "\n// ── Parser State")

par = before_fn_main(par)
parts.append(par.strip())

# ──────────────────────────────────────────────────────────────────────────────
# 3. resolver.ark
# ──────────────────────────────────────────────────────────────────────────────
res = read("resolver.ark")
res = res.replace("use std::host::stdio\n", "", 1)

# Remove NK_* constants and the duplicate AST Node structs
res = remove_between(res,
    "\n// ── AST Node Kind Constants (from parser)",
    "\n// ── Resolve Module")

res = before_fn_main(res)
parts.append(res.strip())

# ──────────────────────────────────────────────────────────────────────────────
# 4. typechecker.ark
# ──────────────────────────────────────────────────────────────────────────────
tc = read("typechecker.ark")
tc = tc.replace("use std::host::stdio\n", "", 1)

# Remove "Resolve Context (duplicated)" section (up to Entry Point)
tc = remove_between(tc,
    "\n// ── Resolve Context (duplicated from resolver for standalone)",
    "\n// ── Entry Point")

tc = before_fn_main(tc)
parts.append(tc.strip())

# ──────────────────────────────────────────────────────────────────────────────
# 5. mir.ark
# ──────────────────────────────────────────────────────────────────────────────
mir = read("mir.ark")
mir = mir.replace("use std::host::stdio\n", "", 1)

# Remove duplicate struct TypeCheckResult, TypedFn, TypeInfo
# These appear between the HIR->MIR section comment and fn lower_to_mir
# Find the section where the structs are and remove just those struct definitions
# (keep the section header comment and fn lower_to_mir)
mir = remove_between(mir,
    "\nstruct TypeCheckResult {\n    error_count: i32,",
    "\nfn lower_to_mir(")

mir = before_fn_main(mir)
parts.append(mir.strip())

# ──────────────────────────────────────────────────────────────────────────────
# 6. emitter.ark
# ──────────────────────────────────────────────────────────────────────────────
emit = read("emitter.ark")
emit = emit.replace("use std::host::stdio\n", "", 1)

# Remove the duplicate MIR struct definitions in "Public API" section
# Keep fn emit_wasm
emit = remove_between(emit,
    "\n// ── Public API (used by driver)",
    "\nfn emit_wasm(")

emit = before_fn_main(emit)
parts.append(emit.strip())

# ──────────────────────────────────────────────────────────────────────────────
# 7. driver.ark
# ──────────────────────────────────────────────────────────────────────────────
drv = read("driver.ark")
drv = drv.replace("use std::host::stdio\n", "", 1)
drv = drv.replace("use std::host::fs\n", "", 1)

# Remove stub section header + TK_ERROR_TAG + token_kind_name (up to print_ast_stderr comment)
drv = remove_between(drv,
    "\n// ── Stub Functions (filled in by later phases)",
    "\n// Stub: print AST to stderr\n")

# Remove fn node_kind_name (duplicate from parser.ark)
# It appears just before "fn main()" -> entry point section
drv = remove_between(drv,
    "\nfn node_kind_name(kind: i32) -> String {\n    if kind == 1 ",
    "\n// ── Entry Point")

drv = before_fn_main(drv)
parts.append(drv.strip())

# ──────────────────────────────────────────────────────────────────────────────
# 8. main.ark (all content except use statements and fn main)
# ──────────────────────────────────────────────────────────────────────────────
main = read("main.ark")
main = main.replace("use std::host::stdio\n", "", 1)
main = main.replace("use std::host::env\n", "", 1)
main = before_fn_main(main)
parts.append(main.strip())

# ──────────────────────────────────────────────────────────────────────────────
# Assemble
# ──────────────────────────────────────────────────────────────────────────────
output = header + "\n\n"
output += "\n\n".join(parts)
output += "\n\n" + new_main

out_path = f"{base}/arukellt.ark"
with open(out_path, 'w') as f:
    f.write(output)

lines = output.count('\n')
print(f"Written {lines} lines to {out_path}")
