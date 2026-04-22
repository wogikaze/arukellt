# Legacy Lowering Path Status

> **Retirement note (2026-04-22, #561)**: This document describes the legacy MIR
> lowering surface as it existed inside the **now-deleted** Rust `crates/ark-mir/`.
> That entire crate was removed in #561; the symbols and file paths below
> (`crates/ark-mir/src/lower/func.rs`, `…/facade.rs`, etc.) no longer exist on
> disk. The selfhost MIR / lowering implementation in `src/compiler/mir.ark` is
> now the source of truth. This file is preserved as a historical record of the
> pre-retirement legacy-fallback state and is no longer the operational status
> document for MIR lowering. See `docs/compiler/pipeline.md` for the current
> picture.

**Updated**: 2026-04-18
**Related issue**: [issues/open/285-legacy-path-deprecation.md](../../issues/open/285-legacy-path-deprecation.md)
**Blocker issue**: [issues/open/508-legacy-path-removal-unblocked-by.md](../../issues/open/508-legacy-path-removal-unblocked-by.md)
**Migration guide**: [legacy-path-migration.md](legacy-path-migration.md)

## Current Pipeline State

All compilation paths — including `MirSelection::CoreHir` and `MirSelection::OptimizedCoreHir`
— currently fall back to the legacy AST lowerer (`lower_to_mir` in
`crates/ark-mir/src/lower/func.rs`).

The call chain for every real compilation is:

```text
Session::compile / compile_with_entry
  └─ run_frontend_for(hint=Some(CoreHir))
       └─ lower_check_output_to_mir(module, core_hir, ...)
            └─ lower_corehir_with_fallback(core_hir, module, ...)  (`lower/facade.rs`)
                 └─ lower_hir_to_mir(core_hir, ...)     ← returns empty MirModule (stub)
                 └─ lower_corehir_via_legacy(module, ...) ← ALWAYS taken (deprecated)
                      └─ lower_hir_fallback(module, ...)
                           └─ lower_to_mir(module, ...)  ← actual lowering (deprecated)
```

`lower_hir_to_mir` is a placeholder: it records CoreHIR structural statistics via
`push_optimization_trace` but always returns an **empty** `MirModule`. As a result,
`lower_corehir_with_fallback` always falls through to `lower_corehir_via_legacy`, which
routes to the legacy AST lowerer.

## Deprecation Status

| Symbol | Location | Status |
|--------|----------|--------|
| `lower_to_mir` | `crates/ark-mir/src/lower/func.rs` | `#[deprecated]` since 0.1.0 |
| `lower_legacy_only` | `crates/ark-mir/src/lower/facade.rs` | `#[deprecated]` since 0.1.0 |
| `lower_prefer_legacy` | `crates/ark-mir/src/lower/facade.rs` | `#[deprecated]` since 0.1.0 |
| `lower_any_to_mir` | `crates/ark-mir/src/lower/facade.rs` | `#[deprecated]` since 0.1.0 |
| `lower_corehir_via_legacy` | `crates/ark-mir/src/lower/facade.rs` | `#[deprecated]` since 0.1.0 |
| `MirSelection::Legacy` | `crates/ark-driver/src/session.rs` | `#[deprecated]` since 0.1.0 |
| `MirSelection::OptimizedLegacy` | `crates/ark-driver/src/session.rs` | `#[deprecated]` since 0.1.0 |
| `--mir-select legacy` CLI flag | `crates/arukellt/src/commands.rs` | Runtime deprecation warning |

See [legacy-path-migration.md](legacy-path-migration.md) for replacement examples and
the staged warning/removal strategy.

## What Cannot Be Removed Yet

The `lower_to_mir` function body (in `func.rs`) is **the only real MIR lowering
implementation**. It processes the typed AST and produces all MIR functions that
the Wasm backend needs. Until `lower_hir_to_mir` is implemented to produce actual
MIR functions from CoreHIR, no compilation fixture will pass without it.

This is tracked in [issues/open/508-legacy-path-removal-unblocked-by.md](../../issues/open/508-legacy-path-removal-unblocked-by.md).

## What Has Changed (2026-04-15)

- `MirSelection::Legacy` and `OptimizedLegacy` are now `#[deprecated]`
- `lower_legacy_only`, `lower_prefer_legacy`, `lower_any_to_mir`,
  `lower_corehir_via_legacy` are now `#[deprecated]`
- The `test` command no longer defaults to `MirSelection::OptimizedLegacy`;
  it unconditionally uses `OptimizedCoreHir` (same behavior, correct label)
- `ARK_USE_COREHIR` env var is no longer needed and is ignored
- All callsites using deprecated variants are guarded with `#[allow(deprecated)]`

## What the Canonical Path Will Look Like

Once `lower_hir_to_mir` is fully implemented:

```text
Session::compile
  └─ run_frontend_for(hint=Some(CoreHir))
       └─ lower_check_output_to_mir
            └─ lower_corehir_with_fallback
                 └─ lower_hir_to_mir  ← returns real MirModule (no fallback)
```

At that point, `lower_to_mir` and all related deprecated functions can be removed.
