---
Status: open
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

- [ ] `Vec::new<i64>`/`Vec::new<f64>` under `use std::collections::vec`
      produce element-typed backing arrays on wasm32-gc
- [ ] push/get/pop on such vecs run identically on wasm32-gc and wasm32
- [ ] positive fixture covering impl-path i64/f64 vec on both backends

## Required evidence

- implementation commit SHA
- fixture + verification gate output

## Notes

Discovered during #874 `Vec::pop` repair (commit 5340919b0). The
no-import builtin path is unaffected — this is the monomorphized impl
body route only.
