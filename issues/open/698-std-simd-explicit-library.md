---
Status: open
Created: 2026-06-26
Updated: 2026-06-26
ID: 698
Track: wasm-feature
Depends on: "686 (wasm-gc-selfhost-implementation), 649 (t4-native-full-lowering)"
Orchestration class: design-ready
Orchestration upstream: None
Blocks v5 exit: no
Priority: 2
Source: ADR-037 design decision 2026-06-26
---

# 698 — std::simd explicit SIMD library API and v128 first-class type

## Summary

Arukellt has no SIMD support. `std::wasm::valtype_v128` exists as an
experimental constant byte only. ADR-037 decides to introduce `std::simd`
as an explicit SIMD library API with v128 as a first-class type, replacing
the rejected Issue #107 (hint-based autovectorization).

This issue tracks the implementation of all decisions recorded in ADR-037:
v128 first-class type, lane-type module API, target-specific lowering,
load/store boundary separation, and std::wasm raw v128 intrinsics.

## Design reference

- **ADR-037**: `docs/adr/ADR-037-std-simd.md` — all 16 decisions
- **Replaces**: `issues/reject/107-runtime-loop-vectorization-hint.md`
- **Wasm 3.0 spec**: v128 is `vectype` (a `valtype`), storable in GC
  struct/array fields via `storagetype`

## Current state

- `std::wasm::valtype_v128` — constant byte `0x7b` only (experimental)
- No v128 type in typechecker / MIR / emitter
- No SIMD opcodes in `emit_opcodes.ark`
- No `std::simd` module
- Issue #107 (loop vectorization hint) — rejected, deferred to v5+
- roadmap-v4.md §4 lists SIMD as non-target (v5+ scope)

## Required work

### Phase 1: v128 first-class type infrastructure

- [x] Add `v128` to typechecker (`src/compiler/typechecker/`)
      — type kind, unification, literal handling
- [x] Add `v128` to MIR type system (`src/compiler/mir/mir_type_info.ark`,
      `src/compiler/mir/mir_opcodes.ark`)
- [x] Add `v128` to Wasm emitter type encoding
      (`src/compiler/wasm/sections_types.ark`, `emit_opcodes.ark`)
- [x] Add v128 locals / params support in emitter
- [ ] Verify v128 storable in GC struct fields and array elements
      (ADR-037 §2, Wasm 3.0 spec compliance) — fixtures created, pending runtime verification

### Phase 2: std::simd portable API

- [x] Create `std/simd/mod.ark` with lane-type module structure:
      - `i8x16` / `u8x16` / `i16x8` / `u16x8`
      - `i32x4` / `u32x4` / `i64x2` / `u64x2`
      - `f32x4` / `f64x2`
      - `v128` (low-level raw type)
- [x] Implement construct: `splat`, `new`/literal, `zero`
- [x] Implement lane access: `extract_lane`, `replace_lane`
- [ ] Implement shuffle / swizzle — **deferred**
- [x] Implement arithmetic: `add`, `sub`, `mul`, float `div`, `sqrt`
- [x] Implement sign / abs: `neg`, `abs`
- [x] Implement comparison: `eq`, `ne`, `lt`, `le`, `gt`, `ge`
- [x] Implement mask / select: `select`, `bitselect`, `any`, `all`, `bitmask`
- [x] Implement bitwise: `and`, `or`, `xor`, `not`, `andnot`
- [x] Implement shift: `shl`, `shr_s`, `shr_u`
- [x] Implement saturating / narrow: `add_sat`, `sub_sat`, `narrow`
- [x] Implement widening / pairwise: `extend`, `extmul`, `extadd_pairwise`
- [x] Implement conversion: `to_i32x4_sat`, `to_f32x4`, `promote`, `demote`
- [x] Register all functions in `std/manifest.toml` with `stability = "experimental"`
- [ ] Syntax support: lane-type module calls (`f32x4::add`) and array literal
      + operator overloading (`let a: f32x4 = [1.0, 2.0, 3.0, 4.0]`) — **deferred**

### Phase 3: std::wasm raw v128 intrinsics

- [x] Add `v128.load` / `v128.store` to `std::wasm` (LinearPtr / LinearSlice only)
- [x] Add `v128.and` / `v128.or` / `v128.xor` / `v128.not` / `v128.andnot`
- [x] Add `v128.bitselect` / `v128.any_true`
- [x] Add `reinterpret` family
- [x] Add `load_splat` / `load_lane`
- [ ] GC Vec ↔ linear memory explicit marshal API (separate from std::simd) — **deferred**

### Phase 4: Target-specific lowering

- [x] T2/T3: emit v128 Wasm SIMD instructions directly
- [x] T1: scalar expansion (no SIMD instructions, same semantics)
- [ ] T4: LLVM native SIMD (`<4 x i32>` etc.) — **split to #699**
- [ ] T5: same as T3 (when T5 backend lands)

### Phase 5: Verification and docs

- [x] Conformance tests for each lane type and operation category
- [x] Lowering tests (v128 emit on T2/T3, scalar expansion on T1)
- [x] GC struct/array v128 field storage tests
- [x] std::simd vs std::wasm boundary tests (no load/store in std::simd)
- [x] Regenerate stdlib docs via `python3 scripts/gen/generate-docs.py`
- [ ] Update `docs/platform/wasm-features.md` with SIMD feature status
- [ ] `python3 scripts/manager.py verify quick` exits 0 — pre-existing failures block this

## Acceptance

- [x] v128 is a first-class type in typechecker, MIR, and Wasm emitter
- [ ] v128 storable in GC struct fields and array elements (Wasm 3.0 spec) — fixtures created, pending runtime verification
- [x] All 11 lane types available in `std::simd` with construct / lane access /
      arithmetic / comparison / mask / bitwise / shift / conversion operations
- [x] `std::simd` has NO load/store API (boundary with std::wasm enforced)
- [x] `v128.load` / `v128.store` / bitwise raw intrinsics in `std::wasm` only
- [x] T2/T3 emit v128 Wasm SIMD instructions
- [x] T1 produces scalar-equivalent computation (no SIMD instructions)
- [ ] T4 uses LLVM native SIMD vector types — **split to #699**
- [x] All `std::simd` entries in manifest have `stability = "experimental"`
- [x] `std::wasm::valtype_v128` remains in `std::wasm` (not moved to std::simd)
- [x] Conformance and lowering tests created
- [ ] `python3 scripts/manager.py verify quick` exits 0 — pre-existing failures (GC array smoke, component WIT, LSP) block this

## Stability promotion criteria (ADR-037 §14)

`std::simd` graduates from `experimental` to `stable` when ALL of:

1. Portable API types / operations / mask / layout are frozen (no breaking changes)
2. Scalar fallback works identically with `+simd128` and `-simd128`
3. GC Vec / struct field / array element SIMD lowering is ADR-002 compliant
4. Raw `std::wasm` API boundary is finalized
5. Conformance tests and lowering tests exist

## Non-goals

- Compiler hint-based autovectorization (`#[vectorize]` etc.) — rejected by
  ADR-037, deferred to v5+ evaluation with `Simd<T,N>` MIR normalization
- Issue #107 revival — this issue is the explicit-API alternative
- T5 backend implementation (tracked by #646)
- Relaxed-SIMD proposal

## Dependencies

- **#686** (wasm-gc-selfhost-implementation): v128 in GC struct/array requires
  GC emitter infrastructure to be in place
- **#649** (t4-native-full-lowering): T4 LLVM native SIMD requires native
  lowering backend (T4 deferred to #699)

## References

- ADR-037: `docs/adr/ADR-037-std-simd.md`
- ADR-002: Wasm GC (v128 GC field storage basis)
- ADR-005: LLVM subordinate (T4 native SIMD as semantics reproduction)
- ADR-007: Target tiers (T1/T2/T3/T4/T5 SIMD availability)
- ADR-014: Stability labels (experimental → stable promotion criteria)
- `issues/reject/107-runtime-loop-vectorization-hint.md` — replaced by this issue
- `std/wasm/mod.ark` — `valtype_v128` (stays in std::wasm)
- [Wasm 3.0 Spec — Types](https://webassembly.github.io/spec/core/syntax/types.html)
- [Wasm 3.0 Spec — Instructions](https://webassembly.github.io/spec/core/valid/instructions.html)
- #699: T4 LLVM native SIMD lowering (split from this issue)
