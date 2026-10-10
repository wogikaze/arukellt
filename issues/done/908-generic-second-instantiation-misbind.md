---
Status: done
Created: 2026-10-10
Updated: 2026-10-10
ID: 908
Track: compiler-internal
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: language-v2
Priority: 1
Source: #695 close audit 2026-10-10 — Vec<String> seq-algorithm probes
---

# 908 — Generic fn second instantiation mis-binds call site on emit (gc invalid wasm / linear silent miscompile)

## Summary

When a generic stdlib function f<T> is also instantiated at i32 by a
stdlib wrapper (binary_search → binary_search_by<i32>,
min_i32/max_i32 → min_by/max_by<i32> in std/seq/mod.ark:40-50), a user
call at a different T mis-binds: the emitted module contains only
f__i32 and the user call site resolves to it.

- wasm32-gc: produces invalid core Wasm
  ("expected i64, found (ref null $type)" for binary_search_by;
  "expected (ref null $type), found (ref null $type)" for min_by).
- wasm32 (linear): emits a call into the i32 instance — silent
  miscompile. i32::cmp runs on String struct pointers, i.e. address
  ordering, not value ordering. It validates and can even look correct
  when allocation order coincides with value order.

## Reproduction

    use std::seq
    fn main() {
        let s: Vec<String> = Vec::new<String>()
        push(s, String_from("a"))
        push(s, String_from("b"))
        assert(seq::binary_search_by(s, String_from("b")) == 1)
        // seq::min_by(s) fails identically
    }

compile --target wasm32-gc → func main fails validation. The emitted
module contains only binary_search_by__i32 — no __String instance is
emitted.

## What is NOT broken (bisected)

- seq::sort_by(Vec<String>) — a proper String instantiation is emitted;
  validates and runs correctly on wasm32-gc and wasm32.
- seq::sort_by(Vec<cmp::Reverse<i32>>) — descending sort works on both
  targets.
- binary_search_by(Vec<i32>, i32) and a user-defined find_first<T: Ord>
  with a single instantiation — work on gc.
- So the failure needs two distinct instantiations of the same generic
  fn, the i32 one coming from a stdlib wrapper.

## Refined diagnosis (2026-10-10 probes)

Trigger pinned: the failure needs **two distinct instantiations of the
same generic fn where the i32 instance is minted inside the imported
std module** (a wrapper body like binary_search → binary_search_by<i32>
or min_i32 → min_by<i32>). Single-module dual instantiation works
(find_first<i32> + find_first<String> both emit and bind); an
imported generic driven to two type args by the user also works
(seq::sort_by<i32> + seq::sort_by<String> both emit as
seq__sort_by__i32 / seq__sort_by__String — note the module-prefixed
names, which sort_by gets because it has no std-internal i32 wrapper).

So the broken case is: pre-existing __i32 instance registered during
the std module's own typecheck + a later user call at T=String. The
user call resolves to __i32 and __String is never emitted.

## Suspected mechanism

- Emit-side call→instance resolution lives in
  src/compiler/mir/lower/call_mono.ark::mir_resolve_mono_call_target:
  type-key candidate (first-arg type) → span-keyed lookup
  (ctx_mono_lookup_call via ctx.mono_call_span_* hash by span_start,
  matched by mono_sources_match + mono_target_matches_source) →
  fallback ctx_resolve_unused_mono_by_name "assign unused mono
  specialization in registration order". That fallback returns the
  first registered instance (__i32) — matching the observed mis-bind —
  and its own comment admits it breaks multi-param generics.
- So: either the typechecker never mints binary_search_by__String /
  records no call-site for it, or the span-keyed lookup misses (e.g.
  source-spelling mismatch for qualified seq::binary_search_by), and
  the order-dependent fallback silently picks __i32.
- Check src/compiler/typechecker/mono_instances.ark
  record_mono_instance / record_mono_call_site and
  module_env_merge.ark for whether a second mint happens and which
  source spelling the span record uses vs what the emit side passes.
- Related: #312 residual (instantiate_type does not walk
  TypeInfo.type_args) and #849 (same invalid-wasm symptom class on
  wasm32-gc func validation).

## Acceptance

- [ ] binary_search_by(Vec<String>, String) compiles to valid
      wasm32-gc (a distinct String instantiation is emitted and bound)
      and returns the correct index at run time.
- [ ] Same for min_by / max_by / binary_search_by_key / lower_bound at
      T = String — no __i32 fallback binding.
- [ ] wasm32 (linear) also emits and binds the String instance (not the
      i32 body running on pointers).
- [ ] Regression fixture exercising Vec<String> through the affected
      generics, with run: + t3-compile: manifest entries.
- [ ] python3 scripts/manager.py verify lane passes.
