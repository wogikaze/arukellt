# Core Ops Registry

> SSOT file: [`docs/data/core-ops.toml`](../data/core-ops.toml)  
> Decision: [ADR-042](../adr/ADR-042-intrinsic-layer-separation.md) D5

## Role

`core-ops.toml` is the machine-readable semantic spine for types and operations
that the compiler may specialise (effect, trap, const-eval, lowering, fallback).

| Concern | Owner |
|---------|-------|
| Semantic ID, type meaning, effect, lowering, fallback symbol | `docs/data/core-ops.toml` |
| Public path, docs, stability, deprecation | `std/manifest.toml` |

Manifest entries **reference** core-ops by `semantic_id` / `type_id`.
They are not generated wholesale from core-ops (see ADR-042 D5 reference model).

```toml
# std/manifest.toml (illustrative)
[[functions]]
name = "add"
module = "std::simd::I32x4"
semantic_id = "simd.i32x4.add"
stability = "experimental"
```

## Fallback identity

`fallback_symbol` is a stable symbol path (or `DefKey` after name resolution).
Compile-local `FunctionId` must not be persisted in the registry.

## Status

Current `schema_version = 1` entries are **scaffold** examples for SIMD.
Resolver / typechecker / MIR / docs generators will consume this file once
implementation starts; until then the shape is normative, the op set is not.
