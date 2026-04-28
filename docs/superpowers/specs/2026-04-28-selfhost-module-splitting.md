# Selfhost Compiler Module Splitting & Subdirectory Support Design

> **Status:** Design Document
> **Drivers:** Selfhost compiler (`src/compiler/`) files exceed 500 lines; module system lacks subdirectory support for user modules.

**Goal:** Split >500-line selfhost compiler files into focused submodules organized in subdirectories, and extend the selfhost module resolver to support subdirectory-style `::` imports with `mod.ark` fallback.

**Architecture:** The Rust reference compiler (`crates/ark-resolve/src/load.rs`) already resolves `foo::bar` → `foo/bar.ark` and falls back to `foo/bar/mod.ark`. The selfhost compiler (`src/compiler/driver.ark`) needs identical logic. Once that's in place, large files are moved into subdirectories with `mod.ark` entry points.

---

## 1. Module Resolution: Selfhost → Rust Parity

### Current behavior (`load_single_module()` in `driver.ark`)

```
<base_dir>/<module_name>.ark                    ← only attempt
```

### Target behavior (Rust-compatible)

```
# For module name "foo::bar::baz":
<base_dir>/foo/bar/baz.ark
<base_dir>/foo/bar/baz/mod.ark                  ← fallback

# For module name "foo" (single segment):
<base_dir>/foo.ark
<base_dir>/foo/mod.ark                           ← fallback
```

### Changes to `src/compiler/driver.ark`

**`load_single_module(module_name, base_dir, state)`:**
1. Normalize: replace `::` with `/` in module_name
2. Try: `<base_dir>/<normalized>.ark`
3. On failure: try `<base_dir>/<normalized>/mod.ark`
4. On failure: report error (no change to error flow)

**No changes needed:**
- `stdlib_path_to_file()` already converts `::` → `/` for stdlib paths
- `load_stdlib_module()` already has `mod.ark` fallback (line 151)
- The Rust compiler already handles this correctly — no changes needed there

---

## 2. File Splitting — Directory Structure

### Overview

```
src/compiler/
├── main.ark                 ← CLI entry (keep, ~700 lines)
├── driver.ark               ← orchestrator (keep, ~500 lines, + resolve logic)
├── analysis.ark             ← keep (<500 lines)
├── component_emitter.ark    ← keep (<500 lines)
├── diagnostics.ark          ← keep (<500 lines)
├── hir.ark                  ← keep (<500 lines)
├── lexer.ark                ← keep (~1200 lines, but logical unit)
├── lsp.ark                  ← keep (~630 lines, but logical unit)
├── resolver.ark             ← keep (~920 lines, but logical unit)
├── typechecker.ark          ← keep (~1340 lines, but logical unit)
│
├── emitter/
│   ├── mod.ark              ← re-export from submodules (abstracted from emitter.ark)
│   ├── ops.ark              ← OP_* opcode constants (~500 lines, 92 functions)
│   ├── sections.ark         ← SEC_* / WASM_* / DATA_* constants (~100 lines)
│   ├── scratch.ark          ← SCRATCH_* register allocation (~200 lines)
│   ├── writer.ark           ← emit_byte/u32/leb128/bytes/f64/string/section (~150 lines)
│   ├── emit.ark             ← emit_wasm_module / emit_mir_inst_ctx / emit_inst (core logic)
│   ├── head.ark             ← (moved from emitter_head.ark)
│   ├── tail.ark             ← emit_wasm / emit_wat (moved from emitter_tail.ark)
│   └── expr.ark             ← (moved from emitter_expr.ark, generated dump, no functions)
│
├── mir/
│   ├── mod.ark              ← re-export from submodules
│   ├── types.ark            ← MirInst, MirBlock, MirFunction, MirModule structs
│   └── lower.ark            ← lower_to_mir, dump_mir, phi logic
│
└── parser/
    ├── mod.ark              ← parse_program, print_ast
    └── types.ark            ← AST node types (struct definitions)
```

### Migration Strategy

Each subdirectory uses `mod.ark` as the entry point so existing `use emitter`, `use mir`, `use parser` remain valid.

**emitter/**: The 13,433-line `emitter.ark`, 2,000-line `emitter_head.ark`, 3,249-line `emitter_tail.ark`, and 6,000-line `emitter_expr.ark` are moved into `emitter/` submodules. The main emission logic stays in `emit.ark`, constants and helpers go into separate files.

**mir/**: The 6,616-line `mir.ark` is split into `types.ark` (structs/type definitions) and `lower.ark` (lowering logic, dump, phi operations).

**parser/**: The 2,403-line `parser.ark` is split into `types.ark` (AST node structs) and `mod.ark` (parsing functions).

**Files kept as-is:** Smaller but still >500-line files like `lexer.ark`, `typechecker.ark`, `resolver.ark`, `lsp.ark`, `main.ark` remain as single files because they are logically cohesive units and splitting them would add more complexity than value.

### Internal Module Dependencies

Modules within a subdirectory can import siblings with simple `use name` (resolved relative to the directory):

```ark
// emitter/emit.ark imports sibling modules:
use ops
use sections
use scratch
use writer

// driver.ark (parent directory) imports submodules via :: syntax:
use emitter::emit
use mir::types
```

---

## 3. Backward Compatibility

- `use emitter` → resolves to `emitter/mod.ark` via the `mod.ark` fallback
- `use mir` → resolves to `mir/mod.ark` via the `mod.ark` fallback
- `use parser` → resolves to `parser/mod.ark` via the `mod.ark` fallback
- `driver.ark` already uses `use emitter` / `use mir` / `use parser` → no changes needed in driver

**No imports in stdlib, tests, or examples need changes.** All existing `use module` statements remain valid because the `mod.ark` fallback preserves them.

---

## 4. Implementation Order

1. **driver.ark**: Update `load_single_module()` with `::` → `/` resolution + `mod.ark` fallback
2. **emitter/**: Create subdirectory, split emitter.ark into ops/sections/scratch/writer/emit submodules, move head/tail/expr, create mod.ark
3. **mir/**: Create subdirectory, split mir.ark into types/lower, create mod.ark
4. **parser/**: Create subdirectory, split parser.ark into types/parser, create mod.ark

---

## 5. Non-Goals

- Rewriting or restructuring the actual logic within any function
- Adding new features or changing compiler behavior
- Changing the Rust reference compiler's module resolution
- Splitting files < 500 lines (lexer, typechecker, resolver, lsp, main)
