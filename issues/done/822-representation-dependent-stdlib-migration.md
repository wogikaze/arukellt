---
Status: done
Created: 2026-07-15
Updated: 2026-07-26
ID: 822
Parent: 729
Track: stdlib
Depends on: "798, 816, 817, 820"
Related: "698, 709, 718, 818, 821, ADR-036, ADR-037, ADR-042, docs/plans/intrinsic-layer-separation"
Orchestration class: blocked
Orchestration upstream: "816, 817, 820"
Blocks v{N}: none
Priority: 2
Source: ADR-042 representation-dependent stdlib migration ownership split
---

# 822 — Representation-dependent and allocating stdlib migration

## Summary

Move Vec/String and other allocation-dependent operations from emitter handlers
to Ark stdlib bodies built on the sealed raw API delivered by #817.

## Scope

- Migrate split/join/replace/repeat/padding/lines, Vec mutation and search,
  and numeric parse/format operation families assigned by the plan.
- Access Vec/String representation only through the sealed raw API.
- Preserve allocation, trap, ordering, and mutation effects declared by CoreOp metadata.
- Add fallback-versus-legacy differential tests for every migrated CoreOp.

## Non-goals

- Do not expose raw representation APIs to general user code.
- Do not implement runtime/WIT host lowering.
- Do not redesign the sealed raw API selected by #817.
- **SIMD portable leftovers** (`simd.i32x4.add` / `simd.i32x4.sub` /
  `simd.f32x4.add`) are **out of scope** for #822 close. They remain
  `legacy_emitter` pending ADR-037 nominal SIMD types + portable scalar
  production lowerings, tracked by **#698**.

## Acceptance

- [x] Assigned representation-dependent CoreOps have Ark implementation symbols and production lowerings
- [x] No assigned (non-SIMD) operation retains a `legacy_emitter` lowering
- [x] Vec/String representation access is confined to the sealed raw API
- [x] Allocation, mutation, trap, and ordering effects match CoreOp metadata
- [x] Differential tests pass for every migrated CoreOp (`test_stdlib_inline`)
- [x] Wave verification: `python3 scripts/manager.py verify lane --gate t3` passes
      (merge-time `verify quick` remains the orchestrator gate)

## Status note (SIMD carve-out)

Assigned #822 work for Vec/String/parse/format/sort is complete. Remaining
repo-wide `legacy_emitter` count is **3**, all SIMD portable ops formally
excluded above and owned by #698 / ADR-037. Closing #822 does not claim those
SIMD CoreOps are migrated.

## Migration progress

- String family, parse/format, seq sort, and concrete Vec i32 helpers migrated
  in earlier wave commits (see git history on `wave/822-repr-stdlib`).
- Generic Vec read/write (**16 → 10**): `len` / `is_empty` / `set` /
  `get_unchecked` (+ typed i64/f64) via sealed raw + MIR rewrite / typecheck
  mono.
- Final Vec tranche (**10 → 3**, SIMD-only residual):
  - LM `raw.array_grow` uses typed element strides (i32/i64/f64/v128).
  - `grow` always publishes logical length (extend or shrink) so pop can
    shrink without a separate set_len call site.
  - `raw.array_new` / `raw.array_set_len` CoreOps added; constructors bind to
    `raw.array_new`.
  - `vec.push` / `push_i64` / `push_f64` → `normal_call` concrete Ark bodies
    (`__core_vec_push_{i32,i64,f64}_impl`) composing grow+set.
  - `vec.get` / `vec.pop` → `normal_call` Option Ark bodies over sealed raw.
  - `vec.Vec_new_*` → `normal_call` / `raw.array_new` bindings.
  - Probes: `probe_vec_push_pop_get_ops`, `probe_vec_new_capacity_ops`.
- Wave path for `legacy_emitter`:
  **31 → 28 → 25 → 23 → 22 → 19 → 18 → 16 → 10 → 3** (SIMD-only residual).

## References

- `issues/open/729-intrinsic-layer-separation.md`
- `issues/done/817-sealed-raw-api-module.md`
- `issues/open/698-std-simd-explicit-library.md` — SIMD leftover ownership
- `issues/open/818-core-op-production-scaffold-exit.md`
- `data/core-ops.toml`
- `docs/adr/ADR-042-intrinsic-layer-separation.md`
- `docs/adr/ADR-037-std-simd.md`
