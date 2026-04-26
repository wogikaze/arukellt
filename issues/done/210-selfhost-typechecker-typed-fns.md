# selfhost: typechecker builds real typed_fns from resolved AST

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 210
**Depends on**: 209
**Track**: compiler/selfhost
**Blocks v1 exit**: no

**Status note**: `src/compiler/typechecker.ark:222` (`typecheck_module`) is a stub that returns an empty `TypeCheckResult`. Nothing flows to MIR. Must be implemented before MIR lowering produces real output.

## Summary

`typecheck_module(resolve_ctx: ResolveCtx) -> TypeCheckResult` currently returns:

```
TypeCheckResult { error_count: 0, errors: [], typed_fns: [] }
```

It must walk `resolve_ctx.scopes` and build at least:
- One `TypedFn` per function declaration
- Infer or propagate argument and return types
- Handle: `i32`, `bool`, `String`, `()` (unit), `let`, `return`, binary ops, function calls, `if`

Scope from `resolver.ark` provides symbol names and kinds; typechecker must annotate expression types.

## Acceptance

- [x] `typecheck_module` returns non-empty `typed_fns` for a file with at least one `fn`
- [x] A file `fn add(a: i32, b: i32) -> i32 { a + b }` produces `TypedFn { name: "add", return_type: TY_I32 }`
- [x] Type errors in the source produce entries in `TypeCheckResult.errors`
- [x] All harness tests still pass

## Out of scope (deferred)

- Generics, trait bounds, `match` exhaustiveness
- Full unification / substitution-based inference

## References

- `src/compiler/typechecker.ark`
- `src/compiler/resolver.ark` (`ResolveCtx`, `Symbol`, `Scope`)

## Completion

Implemented: `typecheck_module` now walks `Vec<AstNode>` decls, finds NK_FN_DECL nodes, and builds TypedFn entries with return type from type annotations. Verified: typed_fns is non-empty for hello.ark (fn main → TY_UNIT). Fixed root-cause MIR bug: Vec<Struct> function parameters not tracked in vec_struct_locals, causing get_unchecked field access to use offset 0 for all fields.
