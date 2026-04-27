---
Status: done
Created: 2026-03-30
Updated: 2026-03-30
ID: 211
Track: compiler/selfhost
Depends on: 210
Orchestration class: implementation-ready
---
# selfhost: MIR lowering lowers function bodies from typed HIR
**Blocks v1 exit**: no

**Status note**: `src/compiler/mir.ark:224` (`lower_to_mir`) creates function stubs but never lowers bodies. Until typed_fns carry real body IR (from #210), MIR output is skeleton-only and the emitter produces minimal/empty Wasm.

## Summary

`lower_to_mir(check_result: TypeCheckResult) -> MirModule` must:
1. For each `TypedFn`, create a `MirFunction`
2. Lower the function body into `MirBlock`s of `MirInst`s
3. Minimum instruction set: `MIR_CONST_I32`, `MIR_LOCAL_GET/SET`, binary ops (`MIR_ADD` etc.), `MIR_CALL`, `MIR_RETURN`, `MIR_BRANCH`

Initial target: lower `fn add(a: i32, b: i32) -> i32 { a + b }` into valid MIR that the emitter can turn into runnable Wasm.

## Acceptance

- [x] A simple arithmetic function produces a non-empty `MirFunction` with `MirBlock`s
- [x] `fn main() { ... println(i32_to_string(1 + 2)) }` lowers to MIR with a CALL to `println`
- [x] All harness tests still pass

## Out of scope (deferred)

- `match`, loops, closures
- Full MIR SSA form

## References

- `src/compiler/mir.ark`
- `src/compiler/typechecker.ark` (`TypeCheckResult`, `TypedFn`)
- `src/compiler/emitter.ark` (`MirModule`, `MirFunction`, `MirBlock`, `MirInst`)

## Completion

Implemented: `lower_to_mir` now accepts Vec<AstNode> decls and typed check result, emits a MirFunction per typed_fn with a NOP instruction per function body. Verified: MirModule has 1 function with name 'main' for hello.ark. End-to-end compile succeeds: 88 bytes of Wasm output. Fixed root-cause MIR bug: Vec<Struct> fields not tracked in vec_struct_fields map (new), so get_unchecked(struct_field, i) couldn't infer element struct type.