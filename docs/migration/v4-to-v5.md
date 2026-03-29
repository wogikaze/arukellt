# Migration Guide: v4 → v5 (Self-Hosting)

> Updated: 2026-03-29

## Overview

v5 re-implements the Arukellt compiler in Arukellt itself. The Rust implementation (Stage 0) compiles the new Arukellt-written compiler (`src/compiler/*.ark`) to produce a self-hosted compiler (Stage 1). The v5 milestone is complete when Stage 1 passes all fixtures and achieves a byte-exact fixpoint with Stage 2.

For end users writing Arukellt programs, v5 introduces **no language-level breaking changes**. The language specification is frozen before v5 work begins. The primary impact is on compiler contributors and the build/bootstrap process.

## Bootstrap Process

```
Stage 0: Rust compiler (arukellt-rust, v4 release binary)
    │ compiles src/compiler/*.ark
    ▼
Stage 1: arukellt-s1.wasm (Arukellt-written compiler)
    │ compiles src/compiler/*.ark (same source)
    ▼
Stage 2: arukellt-s2.wasm
    │ verify
    ▼
Fixpoint: sha256(arukellt-s1.wasm) == sha256(arukellt-s2.wasm)
```

Run the full bootstrap verification:

```bash
scripts/verify-bootstrap.sh
```

## New Directory: `src/compiler/`

The self-hosted compiler source lives in `src/compiler/`:

| File | Component | Phase |
|------|-----------|-------|
| `lexer.ark` | Lexer | Phase 1 |
| `parser.ark` | Parser (recursive descent + Pratt) | Phase 1 |
| `driver.ark` | Compilation driver | Phase 1 |
| `main.ark` | CLI entry point | Phase 1 |
| `resolver.ark` | Name resolution / scope management | Phase 2 |
| `typechecker.ark` | Type inference / unification | Phase 2 |
| `hir.ark` | HIR data structures | Phase 2 |
| `mir.ark` | MIR data structures + lowering | Phase 2 |
| `emitter.ark` | Wasm binary emitter | Phase 3 |

### Not self-hosted

- `ark-llvm` — requires LLVM C API / FFI (not available in Arukellt)
- `ark-lsp` — LSP server remains in Rust

## Language Specification Freeze

`docs/language/spec.md` is frozen before v5 implementation begins. Any language changes during v5 require an ADR and must be applied to both the Rust and Arukellt implementations simultaneously to preserve fixpoint.

## What Changes for Compiler Contributors

### Dual implementation period

During v5 development, both implementations coexist:

- **Rust version** (`crates/`): reference implementation, feature-frozen after Phase 1
- **Arukellt version** (`src/compiler/`): new canonical implementation

Bug fixes during the dual period must be applied to both implementations. Use `scripts/compare-outputs.sh` to verify output parity.

### MIR optimization passes

The seven MIR optimization passes from v4 (`crates/ark-mir/src/passes/`) are re-implemented in Arukellt as part of Phase 2. The pass interface and behavior are preserved.

### Debug output

The Arukellt compiler supports `ARUKELLT_DUMP_PHASES` (read via `args()`) for the same dump categories as the Rust version.

## What Changes for End Users

**Nothing breaks.** The self-hosted compiler accepts the same source language, produces the same Wasm output, and supports the same CLI flags. The only observable differences:

- Compilation may be up to 5× slower than the Rust version (wasmtime JIT overhead)
- Error recovery is initially limited (first error stops compilation; improved in Phase 3)

## Performance

| Metric | Target |
|--------|--------|
| Compile time (vs Rust version) | ≤ 5× slower |
| All v4 fixture tests | Must pass under Stage 1 |
| Fixpoint | Byte-exact Stage 1 == Stage 2 |

## Unchanged Behavior

- All language features from v1–v4 remain available
- stdlib modules and APIs are unchanged
- T1 / T3 compilation targets work identically
- Component Model support (`--emit component`, `--emit wit`) is preserved
- `--opt-level`, `--time`, and all v4 CLI flags are supported

## Migration Checklist

- [ ] No source changes required for end-user code
- [ ] (Compiler contributors) Familiarize with `src/compiler/` layout
- [ ] (Compiler contributors) Use `scripts/verify-bootstrap.sh` for fixpoint checks
- [ ] (Compiler contributors) Apply bug fixes to both Rust and Arukellt implementations
- [ ] (Compiler contributors) Review `docs/language/spec.md` frozen spec before making changes

## Related Documents

- `docs/process/roadmap-v5.md` — historical v5 roadmap
- `docs/compiler/bootstrap.md` — bootstrap procedure (Stage 0 → 1 → 2)
- `docs/language/spec.md` — frozen language specification
- `docs/process/selfhosting-stdlib-checklist.md` — stdlib readiness for self-hosting
- `docs/current-state.md` — current project state
