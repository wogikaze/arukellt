# CoreHIR interface note

This note is implementation-facing only. It freezes the frontend handoff expected by downstream MIR and docs work.

## Compile path status

CoreHIR is **the default** compile path for all CLI commands (`compile`, `build`, `run`).

A desugaring pass (historically `crates/ark-mir/src/opt/desugar.rs`; now in selfhost `src/compiler/mir.ark` after the Rust crate was retired in #561) runs as a pre-optimization step and
converts `IfExpr`, `LoopExpr`, and `TryExpr` operands into statement form (`IfStmt`,
`WhileStmt`, `Assign` with temporaries). This ensures `validate_backend_legal_module` passes.

**CLI flag**: Use `--mir-select legacy` to opt in to the legacy path if needed.
The default is `--mir-select corehir`.

## CoreHIR node inventory

Defined in `crates/ark-hir/src/hir.rs`.

- Top level: `Program`, `Module`, `Item`, `Body`
- Item kinds: `Function`, `Struct`, `Enum`, `Trait`, `Impl`
- Statements: `Let`, `Expr`, `While`, `Loop`, `ForRange`, `ForValues`
- Expressions:
  - literals / locals / globals
  - direct / selected / indirect `Call`
  - builtin unary / binary ops
  - `If`, `Match`, `Block`, `Loop`
  - `Tuple`, `Array`, `ArrayRepeat`
  - `StructInit`, `FieldAccess`, `Index`
  - `Return`, `Break`, `Continue`, `Try`, `Assign`, `Closure`
  - `StringConcatMany`
- Patterns: wildcard, binding, const, tuple, enum, or, struct

Only these sugars are allowed to remain in CoreHIR:
- `Match`
- `Try`
- `ForRange`
- `ForValues`
- `StringConcatMany`

Method-call raw syntax, operator raw syntax, and qualified-import raw syntax must not survive as syntax nodes. They must appear as resolved call targets / selections.

## Validator invariants

Implemented in `crates/ark-hir/src/validate.rs`.

The validator currently enforces:
- every program/module/item/body/expr/pattern expected by downstream code has a source-map entry
- every expr has a non-`Error` type
- every pattern has a non-`Error` type
- direct and selected call targets are non-empty and already resolved
- duplicate bindings inside a single pattern are rejected

Downstream code may rely on successful validation as the minimum integrity gate for CoreHIR input.

## Selection contract

`Selection` records frontend-only dispatch decisions that later stages must not recompute:
- `kind`: function / method / trait-method / operator / from-conversion
- `impl_id`: optional impl owner when known
- `method_item_id`: optional item identity when known
- `generic_substitutions`: chosen substitutions for the selected callee
- `self_ty`: resolved receiver type when applicable
- `resolved_function`: canonical backend-facing callee name

If `CallTarget::Selected` is present, the frontend has already chosen the callee.

## ValueCopy / SharedRef contract

`ValueMode` is attached to params, locals, let bindings, call args, captures, and assign nodes.

- `ValueCopy`: value-semantics copy is intended
- `SharedRef`: alias-preserving shared-reference semantics is intended

Current invariant: do not rewrite `SharedRef` paths into deep copies. This preserves current `let b = a` shared semantics for reference-like values.

## Source-map contract

`crates/ark-hir/src/source_map.rs` stores spans keyed by stable HIR IDs.

Downstream code may assume:
- every validated `ExprId` has an `expr_span`
- every validated `PatternId` has a `pattern_span`
- every `BodyId` and `ItemId` participating in validated HIR has a stored span

If MIR introduces internal nodes without direct source correspondence, attach notes using the nearest originating HIR span rather than fabricating raw parser spans.
