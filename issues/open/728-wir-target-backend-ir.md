---
Status: open
Created: 2026-07-18
Updated: 2026-07-18
ID: 728
Track: compiler-internal
Depends on: none
Related: "714, 646, 649, 680, 727, 668, ADR-007-targets.md, ADR-013-primary-target.md, ADR-008-wasm-gc-post-mvp.md, ADR-005-llvm-scope.md, ADR-008-component-wrapping.md"
Orchestration class: design-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: Comparison with wado WIR layer and ADR-007 multi-target unification
---

# 728 — WIR / backend target IR for ADR-007 multi-target separation

## Summary

ADR-007 defines five target tiers (T1 `wasm32-wasi-p1`, T2 `wasm32-freestanding`, T3 `wasm32-wasi-p2`, T4 `native`, T5 `wasm32-wasi-p3`) and the goal of unifying host interactions on WASI P2/P3 imports. Today, the compiler lowers MIR directly to wasm bytes while target-specific behavior (P1 vs P2 vs freestanding, GC vs linear, core vs component, native scaffold) is encoded by string matching on `--target` and `--wasi-version` scattered across `src/compiler/wasm/`, `src/compiler/mir/lower/`, and `src/compiler/driver/`.

This issue proposes a design investigation: insert a **WIR (Wasm IR) or, more broadly, a low-level target backend IR** between MIR and the final byte emitters. The objective is to separate target-agnostic lowering from target-specific code generation, make the host-function unification required by ADR-007 maintainable, and replace the current `p2_component_wrap.py` post-processing patch with emitter-native canonical ABI glue.

## Evidence source

- `docs/adr/ADR-007-targets.md` lines 25-41, 79-82, 207-210, 356-363, 425-489 (target tiers and host-function unification goals).
- `docs/adr/ADR-013-primary-target.md` (primary = T3, supported/scaffold tier definitions).
- `src/compiler/wasm/emit_target.ark:5-117` (string-based `is_gc_target`, `is_p2_wasi`, `is_freestanding_target`, import/type count helpers).
- `src/compiler/wasm/sections_imports.ark:12-28` (import section branches on target/wasi).
- `src/compiler/wasm/sections_types_sigs.ark:15-37` (WASI type signatures branch on `is_p2_wasi` / `is_component_stub_wasi`).
- `src/compiler/wasm/sections_types_sigs_detail.ark:52-92` (GC target type signature overrides for params/returns).
- `src/compiler/wasm/intrinsic_stdio.ark:17-23`, `intrinsic_clock.ark:11-22`, `intrinsic_random.ark:8-24` (host intrinsics branch on target/wasi for P1/P2/freestanding).
- `src/compiler/wasm/intrinsic_http.ark:17-21`, `intrinsic_sockets.ark:17-31` (GC targets currently emit `unreachable` for HTTP/sockets, leaving host-function unification incomplete).
- `src/compiler/wasm/component_p2_emit.ark:8-14` and `component_p2_run_sections.ark:13-21` (hard-coded P2 command component wrapper).
- `src/compiler/wasm/wasm_sections.ark:16-46` (target/wasi string threaded through every section emitter).
- `src/compiler/mir/lower/ctx_init.ark:11-37` and `ctx_gc_enum.ark:10-11` (`is_gc_target` propagated into MIR lowering).
- `src/compiler/driver/emit.ark:22-23,75-104` (native branch, wasi-version normalization, component target error).
- `src/compiler/driver/target.ark:3-33` (target string classification duplicated).
- `src/compiler/driver/native.ark:14-22` (T4 native scaffold, only `emit_native_scaffold`).

## Current code size

- `src/compiler/wasm/` — 322 `.ark` files, 31,233 lines.
- `src/compiler/wasm/intrinsic_*.ark` — 135 files, 15,811 lines.
- `src/compiler/wasm/sections_*.ark` — 24 files, 2,418 lines.
- `src/compiler/component/` — 143 `.ark` files, 13,178 lines.
- `src/compiler/mir/` — 536 `.ark` files, 23,138 lines (of which `mir/lower/` is 336 files, 16,521 lines).
- `src/compiler/corehir/` — 161 `.ark` files, 5,333 lines.

These numbers show the backend is already large and target-specific conditionals are distributed across many files.

## WIR / WebAssembly context

This design is inspired by the WIR (Wasm IR) layer proposed for wado (wado-lang/wado/docs/wep-2026-02-14-wir-layer.md). The Arukellt-specific target concerns it must address are:

- **Wasm GC** (ADR-008-wasm-gc-post-mvp.md, ADR-035-wasm-gc-implementation.md) — GC type and reference operations (`struct.*`, `array.*`, `ref.cast`) that must be emitted differently from the linear-memory T1 path.
- **Memory model** (ADR-002-memory-model.md) — linear-memory layout, `heap_global`/`memory_id`, and GC-vs-linear representation decisions.
- **Component Model + Canonical ABI** (ADR-008-component-wrapping.md) — the `wasi:cli/run` component wrapper and adapter generation under `src/compiler/component/`.
- **WASI Preview 2 / Preview 3** — the import surface (`wasi:cli/stdout`, `wasi:io/streams`, `wasi:filesystem/types`, etc.) and async-first model that T5 will require.

A WIR or backend target IR would make the compiler's consumption of these targets explicit and per-target rather than string-matched throughout the emitter.

## Goals

1. Evaluate whether a WIR layer between MIR and the wasm byte emitter reduces the number of `is_gc_target` / `is_p2_wasi` / `is_freestanding_target` / `is_native_target` branches.
2. Design a backend IR that can express:
   - core Wasm vs Component Model artifacts
   - WASI P1 vs P2 vs P3 host import surfaces
   - GC-vs-linear memory representation (ADR-002)
   - canonical ABI lift/lower glue (ADR-008, #714)
3. Decide whether the native target (T4) can share the same low-level IR or needs a separate `NativeIR` (per ADR-005: native semantics are subordinate to Wasm).
4. Keep existing T1 and T3 paths working during any transition.

## Non-goals

- Immediate full implementation of T5 or T4.
- Replacing MIR.
- Breaking existing T1/T3 fixture gates.
- Reviving the Rust `ark-llvm` crate.

## Recommendation

A **WIR layer is desirable for the wasm targets** because it would centralize the target-specific import/type/local decisions that currently leak from `src/compiler/wasm/emit_target.ark` into `intrinsics/`, `sections/`, `mir/lower/`, and `component/`. In particular:

- Host function unification (WASI P2/P3 imports) can be expressed as a single `HostCall` WIR operation that is lowered per target.
- Component Model canonical ABI glue can be generated from WIR rather than patched by `scripts/selfhost/p2_component_wrap.py` (#714).
- P1/P2/P3 import tables and type signatures can be derived from the target backend rather than computed by string checks.

It is **not sufficient for T4 native** unless a separate backend IR is used; WIR is wasm-specific. For T4, a tree-shaped low-level IR shared with the wasm backend could simplify lowering, but the final native backend must still emit C/C++/LLVM IR or asm (ADR-005). The design should therefore consider whether to define a generic `BackendIR` with both `Wasm` and `Native` dialects, or keep `WIR` wasm-only and let `native` use a separate path.

## Acceptance

- [ ] Design doc under `docs/adr/` or `docs/compiler/` describing WIR or backend IR structure.
- [ ] Prototype or proof-of-concept that lowers a small subset of MIR (e.g., `stdio::println`) to WIR and then to:
  - T1 P1 core wasm
  - T3 P2 core wasm
  - T3 P2 component wasm (no `p2_component_wrap.py`)
- [ ] Inventory of `is_*_target` / `wasi_version` branches removed or centralized.
- [ ] Decision on whether native T4 uses the same IR or a separate `NativeIR`.
- [ ] Update or close this issue once design is approved; file implementation issues if approved.

## Required verification

```bash
python3 scripts/manager.py verify quick
python3 scripts/check/check-docs-consistency.py
```

## Close gate

Design accepted and implementation issues (or a deliberate "not worth the cost" decision) recorded; no regression in T1/T3 gates.

## References

- `docs/adr/ADR-007-targets.md` (target tiers, host function unification)
- `docs/adr/ADR-013-primary-target.md` (tier definitions)
- `docs/adr/ADR-002-memory-model.md` (linear memory layout, GC-vs-linear representation)
- `docs/adr/ADR-008-wasm-gc-post-mvp.md` (Wasm GC type/operator representation)
- `docs/adr/ADR-035-wasm-gc-implementation.md` (selfhost GC implementation)
- `docs/adr/ADR-005-llvm-scope.md` (native backend constraints)
- `docs/adr/ADR-008-component-wrapping.md` (in-tree component model, canonical ABI)
- `wado-lang/wado/docs/wep-2026-02-14-wir-layer.md` (WIR layer design comparison)
- `issues/open/714-wasi-p2-emitter-native-component-output.md`
- `issues/open/646-t5-wasm32-wasi-p3-target-scaffold.md`
- `issues/open/649-t4-native-full-lowering.md`
- `issues/open/680-target-tier-honesty-audit.md`
- `issues/open/727-arukellt-host-bridge-retirement.md`
- `issues/open/668-p2-native-component-polish.md`
