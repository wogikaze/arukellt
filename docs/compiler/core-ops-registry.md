# Core Ops Registry

> Registry path: [`data/core-ops.toml`](../../data/core-ops.toml)
> Proposed design: [ADR-042](../adr/ADR-042-intrinsic-layer-separation.md) D5
> Migration owner: [issue #798](../../issues/open/798-adr-042-semantic-operation-registry-migration.md)

## Role

`core-ops.toml` is the single source of truth for semantic types,
`SemanticId`, effect, lowering kind, target, and fallback.

`status = "scaffold"` means the compiler does not consume it as an authoritative
input today. Issue #798 owns adoption after ADR-042 acceptance.

| Concern | Owner |
|---------|-------|
| Current semantic registration and lowering | resolver / typechecker / MIR / emitter code |
| Public path, docs, stability, deprecation | `std/manifest.toml` |
| Future semantic metadata | `data/core-ops.toml` (ADR-042) |

`std/manifest.toml` entries may reference `core-ops.toml` via `semantic_id`
(for `[[functions]]`) and `type_id` (for `[[types]]`). Example:

```toml
# std/manifest.toml (illustrative)
[[types]]
name = "String"
type_id = "string"
stability = "stable"

[[functions]]
name = "starts_with"
module = "std::text"
semantic_id = "string.starts_with"
stability = "stable"
```

The `semantic_id` / `type_id` references shown above are proposed, not the
current schema. Issue #798 owns adoption after ADR-042 acceptance.

## Registry schema

The current schema is `schema_version = 2`. See `data/core-ops.toml` for the
reference entries. Each operation has:

- `signature` — `receiver`, `parameters`, `results`, `generic_params`, `constraints`
- `semantics` — `const_evaluable`, overflow/NaN/trap rules
- `effect` — orthogonal attribute set (memory, allocates, may_trap, noreturn,
  external_io, nondeterminism, atomic, volatile)
- `lowering` — `kind` (`normal_call`, `mir_op`, `runtime_call`, `target_intrinsic`)
  and optional `target_id`
- `target` — `capability_predicate` and `fallback_allowed`
- `fallback` — `stable_symbol_path` (or `DefKey` after name resolution)

Compile-local `FunctionId` must not be persisted in the registry.

## Status

Current `schema_version = 2` entries are **scaffold** examples covering
`string.starts_with` and portable SIMD operations.
Resolver / typechecker / MIR / docs generators do not consume this file today.
The schema and operation set are non-normative while ADR-042 remains PROPOSED.
