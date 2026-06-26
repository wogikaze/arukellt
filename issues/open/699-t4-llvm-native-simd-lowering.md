---
Status: open
Created: 2026-06-26
Updated: 2026-06-26
ID: 699
Track: native-feature
Depends on: "649 (t4-native-full-lowering), 698 (std-simd-explicit-library)"
Orchestration class: design-ready
Orchestration upstream: None
Blocks v5 exit: no
Priority: 2
Source: ADR-037 §13, issue #698 Phase 4 split
---

# 699 — T4 LLVM native SIMD lowering for std::simd

## Summary

ADR-037 §13 specifies that T4 (native) targets should use LLVM native SIMD
vector types (`<4 x i32>`, `<2 x i64>`, `<4 x float>`, etc.) as semantics
reproduction for Wasm SIMD operations.

This issue was split out from #698 Phase 4 because the T4 native lowering
backend (#649) is not yet ready. The `std::simd` portable API and v128
first-class type are implemented for T1/T2/T3/T5 in #698, but T4 LLVM
native SIMD lowering remains unimplemented.

## Design reference

- **ADR-037 §13**: LLVM native SIMD as semantics reproduction (not optimization)
- **ADR-005**: LLVM subordinate — native SIMD is "semantics reproduction",
  not "optimization beyond Wasm"
- **#649**: T4 native full lowering (prerequisite backend)

## Required work

- [ ] LLVM emitter: map v128 value type to LLVM vector types
  - `v128` → `<4 x i32>` (default representation)
  - f32x4 operations → `<4 x float>`
  - f64x2 operations → `<2 x double>`
  - i64x2 operations → `<2 x i64>`
- [ ] Map all `__simd_*` intrinsics to LLVM vector intrinsics
  - arithmetic: `add`/`sub`/`mul`/`div` → LLVM vector arithmetic
  - comparison: `eq`/`ne`/`lt`/`le`/`gt`/`ge` → LLVM vector comparison
  - bitwise: `and`/`or`/`xor`/`not` → LLVM vector bitwise
  - lane access: `extractelement`/`insertelement`
  - splat: `broadcast` / `insertelement` to all lanes
- [ ] T4 scalar fallback for operations without direct LLVM vector equivalent
- [ ] Conformance tests on T4 target
- [ ] Verify `std::simd` semantics match between T2/T3 (Wasm SIMD) and T4 (LLVM)

## Acceptance

- [ ] T4 uses LLVM native SIMD vector types (`<4 x i32>` etc.)
- [ ] All `std::simd` operations produce correct results on T4
- [ ] No Wasm-specific SIMD opcodes emitted on T4
- [ ] ADR-005 compliance: no native-specific optimizations beyond Wasm semantics

## Non-goals

- T4 native backend implementation itself (tracked by #649)
- Autovectorization (rejected by ADR-037, deferred to v5+)
- Relaxed-SIMD proposal

## Dependencies

- **#649** (t4-native-full-lowering): T4 LLVM backend must be functional
- **#698** (std-simd-explicit-library): std::simd API and v128 type must be
  implemented (completed for T1/T2/T3/T5)

## References

- ADR-037: `docs/adr/ADR-037-std-simd.md` §13
- ADR-005: LLVM subordinate
- #649: T4 native full lowering
- #698: std::simd explicit library API
