---
Status: done
Created: 2026-07-05
Updated: 2026-07-05
ID: 717
Track: language
Depends on: "039 (done), ADR-031 (decided)"
Orchestration class: ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: Token-cost audit 2026-07-05 — `helpers_core::emit_local_get` verbosity in wasm emitter
---

# 717 — Function-level use destructuring (`use path::mod::{fn_a, fn_b}`)

## Summary

Arukellt's `use` declaration supports module-level destructuring
(`use std::text::{string, fmt}`) but not function-level destructuring
(`use std::text::string::{split, join}`). This forces every cross-module
call to repeat the module prefix: `helpers_core::emit_local_get(w, local)`,
`helpers_core::emit_local_set(w, local)`, etc. The selfhost wasm emitter
alone has **2,000+** `helpers_core::` qualified calls across 70 files,
adding significant token cost with zero semantic value.

The parser already accepts `use path::mod::{name}` syntax and creates
child `NK_USE` nodes (ADR-031, issue #039). The resolver registers the
short name as `SYM_MODULE` — correct for sub-module imports but wrong for
function imports. This issue implements the missing function-level
resolution so bare calls like `emit_local_get(w, local)` work after
`use wasm::intrinsics::helpers_core::{emit_local_get}`.

## Current state

### What works

- `use std::text::{string, fmt}` — sub-module destructuring (issue #039)
- `string::split(s, sep)` — qualified call via module short name
- `helpers_core::emit_local_get(w, x)` — qualified call via use alias

### What doesn't work

- `use std::text::string::{split}` — parsed but `split` registered as
  `SYM_MODULE`, not a function alias
- `split(s, ",")` — bare call fails: `split` is not a module, and the
  function `split` is in the global scope but may be ambiguous (multiple
  modules can define `split`)

### Architecture

The resolution pipeline for a qualified call `module::fn(args)`:

1. **Resolver** (`expr_path.ark`): splits `module::fn` into module part
   and symbol part, checks module is in scope, records ref site
2. **Typechecker** (`call_fn.ark`): looks up `module::fn` in `fn_sigs`
   (exact match); falls through to `call_unknown` → `TY_UNKNOWN` for
   qualified names (permissive)
3. **MIR lowering** (`call_rewrite.ark`): callee_name stays as
   `module::fn`; mono resolution returns it unchanged for non-generic
4. **Wasm emit** (`inst_ctx.ark`): `resolve_fn_index` tries exact match
   `module::fn` (fails), then strips module prefix → `fn` (bare name
   match in function table)

The prefix-stripping at the wasm level is what makes qualified calls
work today. The bare name lookup uses heuristic collision resolution
(`mir_resolve_self_delegate_collision`) when multiple functions share
the same short name.

## Required work

### Phase 1: Use-alias map in resolver

- [x] Add `use_alias_shorts: Vec<String>` and `use_alias_targets:
      Vec<String>` to `ResolveCtx` (`resolver/ctx_record.ark`)
- [x] Add `resolve_ctx_add_use_alias(ctx, short, target)` and
      `resolve_ctx_lookup_use_alias(ctx, short) -> String` accessors

### Phase 2: Register function-level use aliases

- [x] Add `use_module_parent_path(path)` to `resolver/paths.ark` —
      returns everything before the last `::` (e.g.
      `std::text::string` from `std::text::string::split`)
- [x] In `register_use_children` (`resolver/register_use.ark`), for
      each child use item:
      - Compute parent module short name (e.g. `string`)
      - Compute child short name (e.g. `split`)
      - Register use alias: `split` → `string::split`
      - Keep existing `SYM_MODULE` registration for backward
        compatibility (sub-module destructuring still works)

### Phase 3: Rewrite bare calls in resolver

- [x] In `resolve_path` (`resolver/expr_path.ark`), for bare names
      (no `::`):
      - First check local scope — if found as `SYM_LOCAL` or
        `SYM_PARAM`, skip rewrite (locals shadow use aliases)
      - If not a local, check use-alias map — if found, rewrite the
        NK_PATH node text to the alias target (e.g. `split` →
        `string::split`)
      - Fall through to normal qualified path resolution

### Phase 4: Test fixtures

- [x] Add `tests/fixtures/module_import/use_func_destructure.ark` —
      `use std::text::string::{split}` then bare `split(s, sep)`
- [x] Add `tests/fixtures/module_import/use_func_destructure_multi.ark`
      — `use std::text::string::{split, join}` then bare calls
- [x] Wire fixtures into `tests/fixtures/manifest.txt`

### Phase 5: Documentation

- [x] Update `docs/stdlib/module-system.md` — remove "No destructuring
      imports" limitation, document function-level destructuring
- [x] Add example to `docs/current-state.md` if applicable

## Acceptance

- [x] `use std::text::string::{split}` parses, resolves, typechecks,
      and compiles correctly
- [x] Bare call `split(s, sep)` produces the same wasm as
      `string::split(s, sep)`
- [x] Existing sub-module destructuring (`use std::text::{string, fmt}`)
      still works (no regression)
- [x] Existing qualified calls (`string::split(s, sep)`) still work
- [x] Locals/params with the same name as a use alias shadow the alias
- [x] `python3 scripts/manager.py verify quick` — no new failures
      introduced (pre-existing failures unchanged)
- [x] New fixtures compile successfully on selfhost

## Design rationale

**Why rewrite node text in the resolver?** The rest of the pipeline
(typechecker, MIR, wasm) already handles qualified paths via prefix
stripping and heuristic collision resolution. By rewriting the bare
name to a qualified path at the resolver stage, we reuse all existing
machinery without touching the typechecker or MIR.

**Why `string::split` not `std::text::string::split`?** The resolver
only has module short names in scope (e.g. `string`, not `std`). Using
the module short name as the alias target ensures the resolver's module
lookup succeeds. The wasm emitter's prefix stripper handles the rest.

**Why keep `SYM_MODULE` registration?** Sub-module destructuring
(`use std::text::{string, fmt}`) relies on the short name being
registered as `SYM_MODULE`. Removing it would break existing code.
The use-alias map is checked before the scope lookup for bare names,
so function aliases take precedence when they exist.

## Dependencies

- Issue #039 (done) — parser support for `use path::{a, b}`
- ADR-031 (decided) — `use` as the stable import syntax
- No blocking dependencies; can proceed independently

## References

- Parser: `src/compiler/parser/imports_group.ark`
- Resolver: `src/compiler/resolver/register_use.ark`,
  `src/compiler/resolver/expr_path.ark`
- Wasm resolution: `src/compiler/wasm/inst_ctx.ark` (`resolve_fn_index`)
- Existing fixtures: `tests/fixtures/module_import/use_destructure*.ark`
- Limitation doc: `docs/stdlib/module-system.md` lines 106-110
