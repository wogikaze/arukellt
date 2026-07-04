---
Status: open
Created: 2026-07-05
Updated: 2026-07-05
ID: 716
Track: gc-native
Depends on: "686"
Orchestration class: implementation-backfill
Orchestration upstream: "#686 Wasm GC Selfhost Implementation"
Blocks v{N}: none
Priority: 1
Source: GC intrinsic stub audit 2026-07-05 — silent-wrong-result stubs in T3 emitter
---

# GC target intrinsic stub completion (silent-wrong-result backfill)

## Summary

The selfhost wasm emitter has multiple intrinsics whose GC-target code path
returns a **wrong-but-valid** result instead of the correct value. Unlike the
24 host intrinsics that emit `unreachable` on GC targets (which trap loudly),
these stubs **compile cleanly and silently produce incorrect output**, making
them a correctness hazard for any T3 (`wasm32-wasi-p2`) program that touches
`join`, `split`, `contains`, `reverse`, `remove`, `sum`/`product`, `sort`, or
`push_char`.

Issue #686 Phase 4 tracks "all fixtures pass on `wasm32-wasi-p2`" as a single
checkbox, but does not enumerate which intrinsics are stubbed. This issue
provides the explicit inventory so each stub can be fixed and verified
independently, then folded back into #686 Phase 4 as the stubs close.

## Current state — stub inventory

Each row below is a GC-target code path in `src/compiler/wasm/` that returns
a placeholder instead of the correct result. None of the stub files contain
`TODO!` / `FIXME!` / issue references — the limitation is only described in
inline comments like "GC target fallback" / "no-op" / "known limitation".

### Silent-wrong-result stubs (compile OK, runtime wrong)

| # | File | Handler | GC behavior | User-visible symptom |
|---|------|---------|-------------|----------------------|
| 1 | `intrinsic_string_join.ark` | `emit_join_gc` | pushes empty string | `join(parts, sep)` always returns `""` |
| 2 | `intrinsic_string_split.ark` | `emit_split_gc` | pushes empty `Vec<String>` | `split(s, ",")` always returns `[]` |
| 3 | `intrinsic_string_contains.ark` | `emit_contains_String_gc` | pushes `i32 0` | `s.contains("x")` always returns `false` |
| 4 | `intrinsic_string_reverse.ark` | `emit_reverse_String_gc` | drops ref, void no-op | `reverse_String(s)` silently does nothing |
| 5 | `intrinsic_string_format_char.ark` | `emit_push_char_gc` | builds new array, drops it | `push_char(s, c)` silently appends nothing (comment: "known limitation of void push_char on GC targets") |
| 6 | `intrinsic_vec_reverse.ark` | `emit_reverse_gc` | pops vec, pushes it back | `reverse_i32(v)` is a no-op |
| 7 | `intrinsic_vec_remove.ark` | `emit_remove_gc` | pops idx+vec, pushes vec back | `remove_i32(v, i)` is a no-op |
| 8 | `intrinsic_vec_contains.ark` | `emit_contains_i32_gc` | pushes `i32 0` | `contains_i32(v, x)` always returns `false` |
| 9 | `intrinsic_seq_reduce.ark` | `emit_seq_i32_reduce_gc` | pushes initial value | `sum_i32(v)` → `0`, `product_i32(v)` → `0` regardless of input |
| 10 | `intrinsic_sort.ark` | `emit_sort_gc` | pops vec, void no-op | `sort_i32` / `sort_i64` / `sort_f64` silently do nothing |

### Loud-failure stubs (already tracked separately)

The 24 host intrinsics listed in `src/compiler/mir/host_intrinsic_stub_names.ark`
emit `unreachable` on GC targets via `src/compiler/wasm/code_body.ark` lines
29-36. These trap at runtime rather than returning wrong values, so they are
**out of scope** for this issue — they belong to the host-capability rollout
tracked by #675 / #676 / #714.

## Required work

For each stub in the inventory above, implement the GC-native code path using
the existing GC primitives (`array.new`, `array.get`, `array.set`,
`array.copy`, `array.len`, `struct.get`, `struct.set`) already proven in
`emit_vec_push_gc`, `emit_concat_gc`, `emit_split` (T1 path), etc.

- [ ] **Stub 1 — `emit_join_gc`**: allocate result array of total length, copy
      each `Vec<String>` element's bytes followed by `sep` (mirror T1
      `emit_join_setup` / `emit_join_loop` / `emit_join_finish` using
      `array.copy`).
- [ ] **Stub 2 — `emit_split_gc`**: scan source, push substring slices into a
      GC `Vec<String>` (mirror T1 `emit_split_scan_loop`).
- [ ] **Stub 3 — `emit_contains_String_gc`**: byte-scan haystack for needle,
      push `i32 1` / `i32 0` (mirror T1 `emit_contains_scan_loop`).
- [ ] **Stub 4 — `emit_reverse_String_gc`**: allocate new array, copy bytes
      end-to-start (mirror T1 `emit_reverse_swap_loop` as a copy-into-new
      since GC arrays are fixed-size).
- [ ] **Stub 5 — `emit_push_char_gc`**: the current code builds a new array
      but drops it. Either (a) change `push_char` to return the new String
      (API change, affects prelude signature) or (b) document that
      `push_char` is unsupported on GC and route callers through
      `concat(s, char_to_string(c))` in std. Decide which before implementing.
- [ ] **Stub 6 — `emit_reverse_gc` (Vec)**: allocate new array, copy elements
      end-to-start, swap into vec struct (mirror T1 `emit_reverse_loop`).
- [ ] **Stub 7 — `emit_remove_gc`**: shift elements down via `array.copy`,
      decrement len, shrink array (or allocate smaller and swap).
- [ ] **Stub 8 — `emit_contains_i32_gc`**: scan vec, push `i32 1` / `i32 0`
      (mirror T1 `emit_contains_i32_loop`).
- [ ] **Stub 9 — `emit_seq_i32_reduce_gc`**: scan vec accumulating via
      `i32.add` / `i32.mul` (mirror T1 `emit_seq_i32_reduce`).
- [ ] **Stub 10 — `emit_sort_gc`**: in-place insertion sort via `array.get` /
      `array.set` (mirror T1 `emit_sort_insertion`). Note: GC arrays are
      fixed-size, so in-place element swap is fine; no resize needed.
- [ ] For each completed stub, add a T3 fixture if one does not already exist
      under `tests/fixtures/t3/` and wire it into `tests/fixtures/manifest.txt`.
- [ ] After all stubs close, verify #686 Phase 4 "all fixtures pass on
      `wasm32-wasi-p2`" checkbox can be ticked.

## Acceptance

- [ ] All 10 silent-wrong-result stubs replaced with correct GC-native
      implementations (or, for `push_char` only, explicitly documented as
      unsupported on GC with std-side workaround).
- [ ] Each fixed stub has a T3 fixture that exercises the intrinsic and
      checks the result on `wasm32-wasi-p2`.
- [ ] `python3 scripts/manager.py verify --full` reports 0 T3 failures
      attributable to these intrinsics.
- [ ] #686 Phase 4 "GC 全フィクスチャ通過" checkbox updated to reflect
      this issue's completion.
- [ ] No T1 (`wasm32-wasi-p1`) regression: `python3 scripts/manager.py verify quick` exits 0.

## References

- Parent / depends on: #686 (Wasm GC Selfhost Implementation) — Phase 4
- ADR-035: Wasm GC Implementation Plan
- `docs/design/gc-implementation-plan.md` Phase 4 verification table
- Stub source files: `src/compiler/wasm/intrinsic_string_*.ark`,
  `src/compiler/wasm/intrinsic_vec_*.ark`, `src/compiler/wasm/intrinsic_seq_reduce.ark`,
  `src/compiler/wasm/intrinsic_sort.ark`, `src/compiler/wasm/intrinsic_string_format_char.ark`
- Loud-failure host stubs (out of scope): `src/compiler/mir/host_intrinsic_stub_names.ark`,
  `src/compiler/wasm/code_body.ark` lines 29-36
