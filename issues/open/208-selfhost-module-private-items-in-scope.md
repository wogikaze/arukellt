# selfhost: module private items must be visible within module scope

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 208
**Depends on**: —
**Track**: compiler/selfhost
**Blocks v1 exit**: no

**Status note**: Blocking selfhost compiler E2E wiring. `use lexer` from `driver.ark` fails because `bind_public_module` skips private items, so pub functions that call private helpers (e.g., `tokenize` → `Lexer_new`) get "unresolved name" errors in the merged module.

## Summary

When a module is loaded via `use`, `bind_public_module` adds only `pub` items to the global symbol table. `resolved_program_to_module` likewise copies only `pub` items into the merged AST.

This breaks any `pub` function whose body references a private helper, because the private helper is absent from both the symbol table and the merged module.

## Root cause

`crates/ark-resolve/src/analyze.rs`:
```rust
for loaded in graph.loaded.values() {
    bind_public_module(&loaded.ast, &mut symbols, global_scope, sink);
}
```
`crates/ark-resolve/src/resolve.rs` (`resolved_program_to_module`):
```rust
if is_pub { module.items.push(item.clone()); }
```

## Fix

1. In `analyze_program`, call a new `bind_module_skip_dup` (includes private, skips duplicates) instead of `bind_public_module` for user-local modules.
2. In `resolved_program_to_module`, include ALL items from user-local modules (not just pub), skipping duplicate names.

Name conflicts across modules (e.g., `Token` defined in both `lexer.ark` and `parser.ark`) are handled by `skip_duplicates=true` — first definition wins, identical layouts ensure structural compatibility.

Standalone `main()` functions in each module must be renamed or removed to avoid the duplicate-`main` conflict.

## Acceptance

- [ ] `arukellt check src/compiler/driver.ark` produces no unresolved-name errors related to cross-module private helpers
- [ ] All existing harness tests still pass (421 PASS)
- [ ] `src/compiler/lexer.ark` can still be run standalone: `arukellt run src/compiler/lexer.ark`

## References

- `crates/ark-resolve/src/analyze.rs`
- `crates/ark-resolve/src/bind.rs`
- `crates/ark-resolve/src/resolve.rs` (`resolved_program_to_module`)
- `src/compiler/lexer.ark` (private helpers called by pub `tokenize`)
- `tests/fixtures/modules/use_local_module/` (working example — helpers.ark has no private items)
