# Core Ops Registry

> Future registry scaffold: [`docs/data/core-ops.toml`](../data/core-ops.toml)
> Proposed design: [ADR-042](../adr/ADR-042-intrinsic-layer-separation.md) D5
> Migration owner: [issue #798](../../issues/open/798-adr-042-semantic-operation-registry-migration.md)

## Role

`core-ops.toml` sketches a future machine-readable semantic spine. Its
`status = "scaffold"` means the compiler does not consume it as an authoritative
input today.

| Concern | Owner |
|---------|-------|
| Current semantic registration and lowering | resolver / typechecker / MIR / emitter code |
| Public path, docs, stability, deprecation | `std/manifest.toml` |
| Proposed future semantic metadata | `docs/data/core-ops.toml` scaffold (ADR-042) |

Manifest `semantic_id` / `type_id` references shown below are proposed, not the
current schema. Issue #798 owns adoption after ADR-042 acceptance.

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
Resolver / typechecker / MIR / docs generators do not consume this file today.
The schema and operation set are non-normative while ADR-042 remains PROPOSED.
