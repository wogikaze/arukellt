# selfhost: MIR lowering lowers function bodies from typed HIR

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 211
**Depends on**: 210
**Track**: compiler/selfhost
**Blocks v1 exit**: no

**Status note**: `src/compiler/mir.ark:224` (`lower_to_mir`) creates function stubs but never lowers bodies. Until typed_fns carry real body IR (from #210), MIR output is skeleton-only and the emitter produces minimal/empty Wasm.

## Summary

`lower_to_mir(check_result: TypeCheckResult) -> MirModule` must:
1. For each `TypedFn`, create a `MirFunction`
2. Lower the function body into `MirBlock`s of `MirInst`s
3. Minimum instruction set: `MIR_CONST_I32`, `MIR_LOCAL_GET/SET`, binary ops (`MIR_ADD` etc.), `MIR_CALL`, `MIR_RETURN`, `MIR_BRANCH`

Initial target: lower `fn add(a: i32, b: i32) -> i32 { a + b }` into valid MIR that the emitter can turn into runnable Wasm.

## Acceptance

- [ ] A simple arithmetic function produces a non-empty `MirFunction` with `MirBlock`s
- [ ] `fn main() { ... println(i32_to_string(1 + 2)) }` lowers to MIR with a CALL to `println`
- [ ] All harness tests still pass

## Out of scope (deferred)

- `match`, loops, closures
- Full MIR SSA form

## References

- `src/compiler/mir.ark`
- `src/compiler/typechecker.ark` (`TypeCheckResult`, `TypedFn`)
- `src/compiler/emitter.ark` (`MirModule`, `MirFunction`, `MirBlock`, `MirInst`)
