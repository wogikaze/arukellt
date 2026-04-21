# Legacy Path Deprecation Migration

> Current state first: the canonical pipeline is described in [../current-state.md](../current-state.md).
> This page documents how to migrate off the deprecated legacy MIR-lowering path and what warnings to expect while that path still exists.

## What Is Deprecated

Arukellt now treats the legacy AST-to-MIR lowering path as a compatibility path, not
the primary compilation route.

The deprecated surface is:

| Surface | Current status | Replacement |
|---------|----------------|-------------|
| `--mir-select legacy` | Deprecated CLI selection | Omit `--mir-select`, or use `--mir-select corehir` / `--mir-select optimized-corehir` when you need an explicit selector |
| `MirSelection::Legacy` | `#[deprecated]` since 0.1.0 | `MirSelection::CoreHir` |
| `MirSelection::OptimizedLegacy` | `#[deprecated]` since 0.1.0 | `MirSelection::OptimizedCoreHir` |
| `lower_legacy_only` / `lower_prefer_legacy` / `lower_any_to_mir` / `lower_corehir_via_legacy` | `#[deprecated]` since 0.1.0 | `lower_check_output_to_mir` |
| `lower_to_mir` | Deprecated implementation detail | Real CoreHIR lowering once `lower_hir_to_mir` is complete |

`ARK_USE_COREHIR` is no longer part of the migration story. The environment variable
is ignored because CoreHIR selection is already the default.

## What Is Not Deprecated

The language semantics are not split into "legacy syntax" and "new syntax" here.
This deprecation is about the compiler's MIR lowering route and the selectors that
force the old path.

- `check`, `compile`, and `run` already default to the CoreHIR-labelled path
- `test` already uses `MirSelection::OptimizedCoreHir`
- Existing programs should stop requesting `legacy`; they do not need source-level rewrites

## Migration Examples

### CLI

Before:

```bash
arukellt compile app.ark --mir-select legacy
arukellt run app.ark --mir-select legacy
arukellt check app.ark --mir-select legacy
```

After:

```bash
arukellt compile app.ark
arukellt run app.ark
arukellt check app.ark
```

If you need an explicit non-legacy selector for debugging or reproducibility, use:

```bash
arukellt compile app.ark --mir-select corehir
arukellt test app.ark --mir-select optimized-corehir
```

### Rust Call Sites

Before:

```rust
let mir = session.lower_mir(MirSelection::Legacy)?;
let optimized = session.lower_mir(MirSelection::OptimizedLegacy)?;
```

After:

```rust
let mir = session.lower_mir(MirSelection::CoreHir)?;
let optimized = session.lower_mir(MirSelection::OptimizedCoreHir)?;
```

### Internal Lowering Entry Points

Before:

```rust
let mir = lower_any_to_mir(module, checker, sink)?;
```

After:

```rust
let mir = lower_check_output_to_mir(module, core_hir, sink)?;
```

## Warning Strategy

Arukellt uses two warning layers during the deprecation window.

1. CLI users who pass `--mir-select legacy` receive a runtime deprecation warning.
2. Rust/internal call sites using legacy selectors or helper functions hit Rust
   `#[deprecated]` warnings at compile time.

The strategy is intentionally staged:

- Stage 1: mark legacy selectors and helper APIs deprecated
- Stage 2: document the migration path and keep the compatibility fallback available
- Stage 3: remove the fallback after the real CoreHIR lowerer exists and fixture parity is proven

Removal is not date-driven alone. It is blocked by [../../issues/open/508-legacy-path-removal-unblocked-by.md](../../issues/open/508-legacy-path-removal-unblocked-by.md), because `lower_hir_to_mir` still returns empty MIR.

## Recommended Migration Order

1. Remove `--mir-select legacy` from scripts, CI jobs, and local aliases.
2. Replace deprecated `MirSelection::*Legacy` variants in Rust call sites.
3. Replace deprecated lowerer helper calls with `lower_check_output_to_mir`.
4. Keep verification on the normal quick gate until issue #508 is resolved.

## Related Docs

- [legacy-path-status.md](legacy-path-status.md)
- [pipeline.md](pipeline.md)
- [ir-spec.md](ir-spec.md)
- [../../issues/open/285-legacy-path-deprecation.md](../../issues/open/285-legacy-path-deprecation.md)
- [../../issues/open/508-legacy-path-removal-unblocked-by.md](../../issues/open/508-legacy-path-removal-unblocked-by.md)
