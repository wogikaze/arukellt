---
Status: done
Created: 2026-09-27
Updated: 2026-09-27
ID: 889
Parent: 852
Track: language-v2
Depends on: "874"
Related: "ADR-058, RFC-011, #852, #822, #872, #874"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 2
Source: "#874 receipt — impl-path Vec::new<T> element specialization gap on wasm32-gc"
---

# 889 — impl-path Vec::new<T> loses element specialization on wasm32-gc

## Scope

When `use std::collections::vec` is present, `Vec::new<T>` resolves to
the inherent impl body `raw::raw_array_new(0)` instead of the pseudo
constructor. On `wasm32-gc` the backing array then keeps the raw-ctor
i32 default instead of the mono element type, so
`Vec<i64>`/`Vec<f64>` built this way trap later
(`out of bounds array access` inside `__core_vec_push_i64_impl` /
`vec_pop`) while `Vec<i32>` and `Vec<String>` work.

Evidence (probes in `.build/probe/`, this branch):

- `p_imp3`: `Vec<i32>` + import — GC and LM both print `2 9 9`.
- `p_imp_str`: `Vec<String>` + import — GC prints `2 cd` correctly.
- `p_imp2` / `p_push64`: `Vec<i64>` + import — GC traps
  `out of bounds array access` on the first `v.push(42_i64)`;
  LM runs correctly (`1 42`).
- Without the import, `Vec<i64>` works on both backends (pseudo ctor
  path), so the gap is specific to the imported impl-body lowering.

The `raw_ctor_fn_return_vec_type_name` machinery already exists for the
same shape (#872, `__core_vec_new_f64_impl`); the fix likely lives where
mono'd `Vec__new__i64`/`Vec__new__f64` bodies stamp their enclosing
return type, or where `raw.array_new` inside them resolves its array
element type.

## Acceptance

- [x] `Vec::new<i64>`/`Vec::new<f64>` under `use std::collections::vec`
      produce element-typed backing arrays on wasm32-gc
- [x] push/get/pop on such vecs run identically on wasm32-gc and wasm32
- [x] positive fixture covering impl-path i64/f64 vec on both backends

## Required evidence

- implementation commit SHA
- fixture + verification gate output

## Notes

Discovered during #874 `Vec::pop` repair (commit 5340919b0). The
no-import builtin path is unaffected — this is the monomorphized impl
body route only.

## Progress receipt — 2026-09-27 (issue closing)

Commit: `757fdf98b` on `docs/adr-058-language-v2-semantics`.

Corrected diagnosis: the impl-path ctor already emitted element-typed
backing arrays (`array.new_default` of the i64/f64 array type plus the
matching vec struct). The trap came from
`emit_push_numeric_gc_grow`, which emits bare `cap*2` without the
`cap == 0 → 8` clamp that `emit_gc_growth_check` gained in #807.
`raw::raw_array_new(0)` honors the explicit zero hint on wasm32-gc, so
the first `Vec<i64>`/`Vec<f64>` push grew to a second 0-length array
and `array.set` trapped. The pseudo-ctor path never saw cap 0 because
`emit_vec_new_layout_gc` defaults to capacity 8.

Second gap fixed in the same acceptance clause: linear-memory
`emit_vec_get` was i32-only — it built the Some record with a 4-byte
payload and i32 element stride, silently truncating i64/f64 elements on
both impl and pseudo-ctor paths (`get(0)` on `Vec<i64>` only worked
by accident via the low word). LM vec_get now dispatches on
`lm_get_unchecked_elem_kind` and writes the 12-byte
`{tag:i32, payload:8B}` Some record — the same layout
`emit_vec_pop_some_wide` and `emit_i64_ok_result` use (#874), so
match payload reads stay consistent.

Evidence:

- Implementation commit `757fdf98b`
  (`src/compiler/wasm/intrinsic_vec_numeric.ark`,
  `src/compiler/wasm/intrinsic_vec_access.ark`).
- Positive fixture
  `tests/fixtures/collections/vec_impl_path_numeric.ark` exercises
  `Vec::new<i64>`, `Vec::new<f64>`, `Vec::with_capacity<i64>(0)`,
  push/get(0)/get(1)/pop/len under `use std::collections::vec`;
  registered `run:` + `t3-run:`. Verified output
  `2 42 7 7 1 2 2.25 9` is byte-identical on the wasm32-gc component
  and the wasm32 core module.
- Probe matrix re-verified on the final s2: impl-path i64 push+get,
  i32, and String vecs produce identical results on both backends;
  f64 `vec_get` probe now returns `1.5`/`2.25` on LM where it
  previously printed `0`/`0.00000000000000000`.
- `python3 scripts/manager.py selfhost build-compiler`: PASS 1/1 on
  the final source.
- `python3 scripts/manager.py verify lane`: EXIT=0 (comment policy,
  semantic-debt, lint smoke, touched-code ratchet, fmt, lint ratchet,
  quality contract).
