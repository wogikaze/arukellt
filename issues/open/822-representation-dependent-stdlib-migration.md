---
Status: open
Created: 2026-07-15
Updated: 2026-07-16
ID: 822
Parent: 729
Track: stdlib
Depends on: "798, 816, 817, 820"
Related: "709, 718, 818, 821, ADR-036, ADR-042, docs/plans/intrinsic-layer-separation"
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
  HashMap/HashSet, and numeric parse/format operation families assigned by the plan.
- Access Vec/String representation only through the sealed raw API.
- Preserve allocation, trap, ordering, and mutation effects declared by CoreOp metadata.
- Add fallback-versus-legacy differential tests for every migrated CoreOp.

## Non-goals

- Do not expose raw representation APIs to general user code.
- Do not implement runtime/WIT host lowering.
- Do not redesign the sealed raw API selected by #817.

## Acceptance

- [ ] Assigned representation-dependent CoreOps have Ark implementation symbols and production lowerings
- [ ] No assigned operation retains a `legacy_emitter` lowering
- [ ] Vec/String representation access is confined to the sealed raw API
- [ ] Allocation, mutation, trap, and ordering effects match CoreOp metadata
- [ ] Differential tests pass for every migrated CoreOp
- [ ] `python3 scripts/manager.py verify quick` passes

## Migration progress

- The current String tranche moves `len`, `char_at`, `slice`, `concat`,
  `starts_with`, `ends_with`, `contains`, `index_of`, ASCII case conversion,
  trim variants, `repeat`, `replace`, `split`, `join`, `lines`, padding,
  storage construction/clone/from-bytes, byte equality, and `chars` to private
  Ark bodies in `std/collections/string.ark`. Each semantic CoreOp uses
  `normal_call` lowering with an implementation symbol and is exercised at O0
  and O1.
- Two-argument compatibility padding and three-argument explicit-fill padding
  are separate CoreOps. This removes the previous alias/signature collision
  between prelude `pad_left` / `pad_right` and `std::text` padding.
- Ark bodies access String representation only through the sealed
  `std::core::raw` boundary. `raw.string_len_bytes` and
  `raw.string_byte_at_unchecked` are separate internal `target_raw` CoreOps;
  the backend owns only those representation primitives.
- Vec migration has started with concrete operations in `std/prelude.ark` and
  `std/seq/mod.ark`. `contains_i32`, the i32 reductions, binary search,
  equality counting, and i32 min/max use concrete private Ark bodies behind
  the existing public compatibility entry points. Generic `len`, `is_empty`, `push`,
  `set`, and `get_unchecked` remain emitter-owned because CoreOp fallback
  rewriting currently happens after type checking and therefore cannot create
  a concrete monomorphized fallback body. Treating the unspecialized
  `Vec<T>` body as production causes invalid GC reference/value types.
- `raw.array_grow` and `raw.array_set_unchecked` are target-raw CoreOps.
  Their LM and GC handlers preserve the raw Vec layout, grow capacity,
  copy existing elements, extend logical length, and permit a subsequent
  unchecked write. The sealed raw differential fixture exercises both targets.
- Concrete `Vec<i32>` and `Vec<String>` reverse operations use separate Ark
  implementation symbols so GC lowering never has to infer a generic element
  representation.
- Concrete i32 sequence allocation operations (`take`, `skip`, and stable
  `unique`) and i32 in-place sort now execute Ark bodies. Their O0/O1
  differential probes verify allocation, ordering, and mutation results.
- Integer formatting for `i32` and `i64`, including both minimum values, now
  uses private Ark bodies in `std/core/scalar_format.ark`. The i64 body does
  not call the still-runtime-owned integer conversion CoreOp.
- `scripts/tests/test_stdlib_inline.py` runs exact-result checks for the
  migrated String and Vec/seq operations in both fallback and optimized
  builds. After the format_f64 / push_char tranche `legacy_emitter` is
  **23** (wave path: 31 → 28 → 25 → 23).
- Wave `wave/822-repr-stdlib` migrated `text.format_bool`, `text.char_to_string`,
  and `core.range_new` to `normal_call` with sealed-raw / Ark bodies
  (`__core_format_bool_impl`, `__core_char_to_string_impl`,
  `__core_range_new_impl`). Bare prelude `format_bool` now binds to
  `text.format_bool`. O0/O1 differential probes cover the three ops.
- Same wave then migrated `math.sqrt`, `scalar.f64_bits_lo`, and
  `scalar.f64_bits_hi` to `normal_call` via sealed `raw.f64_sqrt` /
  `raw.f64_bits_*` (`target_intrinsic`) and
  `__core_math_sqrt_impl` / `__core_scalar_f64_bits_*_impl`. O0/O1 probes
  `probe_sqrt_ops` and `probe_f64_bits_ops` cover them.
- Next tranche migrated `text.format_f64` and `text.push_char` to
  `normal_call` via sealed `raw.f64_to_string` / `raw.string_push_char`
  and `__core_format_f64_impl` / `__core_push_char_impl`. O0/O1 probes
  `probe_format_f64_ops` and `probe_push_char_ops` cover them.
- `text.f32_to_string` stays `legacy_emitter`: typed `f32` literals currently
  typecheck/lower as `f64`, and the legacy `f32↔i32` bit bridge does not
  match typed `f32` params, so a honest differential probe cannot land yet.
- `parse.parse_*` stay `legacy_emitter`: prelude `Result` and
  `convert::parse_*` `Option` aliases still collide on one CoreOp.
- `seq.sort_i64` / `seq.sort_f64` stay `legacy_emitter`: an Ark insertion-sort
  body using generic `set` still emits i32 stores for i64/f64 elements
  (invalid wasm). Needs monomorphic set helpers or a typed `set` fix first.
- Remaining `legacy_emitter` (23): Vec mutation/allocation family
  (`vec.*`), `text.f32_to_string`, numeric parse (`parse.parse_*`),
  `seq.sort_{i64,f64}`, and SIMD portable leftovers (`simd.i32x4.*`,
  `simd.f32x4.add`). Generic Vec mutation additionally requires the
  fallback resolver to select or synthesize a call-site-specialized
  implementation before those CoreOps can leave `legacy_emitter`.
- i64/f64 monomorphic Vec helper aliases are present in the frozen registry but
  are not resolver-reachable source APIs, so they cannot yet receive required
  differential tests.
- **Close stance:** do not move #822 to done while assigned Vec/parse/f32/
  SIMD/sort families remain `legacy_emitter`. Prefer follow-up issues for
  typed store / parse signature / f32 literal+ABI rather than a false close.

## References

- `issues/open/729-intrinsic-layer-separation.md`
- `issues/done/817-sealed-raw-api-module.md`
- `issues/open/818-core-op-production-scaffold-exit.md`
- `issues/open/820-stdlib-only-inliner.md`
- `data/core-ops.toml`
- `docs/adr/ADR-042-intrinsic-layer-separation.md`
