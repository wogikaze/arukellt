# f32 ローカル変数の MIR lowerer 対応 (F32 locals tracking)

**Status**: done
**Created**: 2026-04-03
**Updated**: 2026-04-10
**ID**: 486
**Depends on**: 040
**Track**: compiler
**Blocks v1 exit**: no

---

## Background

Issue #040 (scalar type completeness) added `f32` support including suffix literals and
conversion functions (`f32_to_f64`). However, using `f32` as a local variable type
(e.g., `let x: f32 = 1.5f32`) currently fails because the MIR lowerer in
`crates/ark-mir/src/lower/func.rs` does not track F32 locals correctly — only F64
locals are tracked.

This was identified as a pre-existing limitation during #040's implementation and is
explicitly out of scope for that issue.

## Summary

Extend the MIR lowerer in `crates/ark-mir/src/lower/func.rs` to properly allocate and
track `f32` (F32) typed local variables, analogous to the existing `f64` / `i64`
tracking added for #040.

## Why this is a separate issue

The `f32` local variable gap is a separate implementation concern from scalar conversion
functions. It was deferred from #040 to keep that issue's scope tight. Fixing it touches
only the lowerer and Wasm emission, not the stdlib prelude.

## Visibility

internal-only (compiler correctness; user-visible indirectly via `let x: f32 = ...` code)

## Primary paths

- `crates/ark-mir/src/lower/func.rs` — local variable allocation and F32 locals tracking
- `crates/ark-wasm/src/emit/t1/operands.rs` — Wasm emission for F32 local loads/stores

## Allowed adjacent paths

- `tests/fixtures/scalar/` — add f32_local.ark fixture
- `tests/fixtures/manifest.txt` — register new fixture

## Non-goals

- f32 arithmetic operations beyond what is already in #040
- f64 locals (already working)
- other scalar types

## Acceptance

1. `let x: f32 = 1.5f32` compiles and runs without a lowerer panic or miscompile
2. A fixture `tests/fixtures/scalar/f32_local.ark` exists and passes with `run:` kind
3. `cargo test -p arukellt --test harness` exits 0 with no regressions

## Required verification

- `grep -n "F32\|f32_local" crates/ark-mir/src/lower/func.rs` shows F32 local tracking code
- `cat tests/fixtures/scalar/f32_local.ark` shows a valid `let x: f32` usage
- `cargo test -p arukellt --test harness` passes

## Close gate

- `crates/ark-mir/src/lower/func.rs` has F32 locals allocation (grep evidence)
- `tests/fixtures/scalar/f32_local.ark` exists and is in `manifest.txt` as `run:` entry
- Full harness passes (19/19 verify-harness checks, all fixture tests)

## Evidence to cite when closing

- Modified lines in `crates/ark-mir/src/lower/func.rs`
- `tests/fixtures/scalar/f32_local.ark` content
- `cargo test -p arukellt --test harness` output

## False-done risk if merged incorrectly

- F32 local allocation code added but not wired (dead code path)
  → acceptance 1 requires the fixture to actually RUN and produce correct output
- Only parse-only fixture added instead of `run:` fixture
  → acceptance 2 explicitly requires `run:` kind
