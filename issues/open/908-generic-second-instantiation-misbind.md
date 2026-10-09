---
Status: open
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

## Suspected area (not yet root-caused)

- src/compiler/typechecker/call_generic_fn.ark:55
  (mono::record_mono_instance), mono_instances.ark, and the emit-side
  instance enumeration / call-site → mangled-name mapping
  (record_mono_call_site).
- Possibly the #312 residual: instantiate_type does not walk
  TypeInfo.type_args, so Vec<T> parameter positions may fail to bind
  T = String, leaving the call unminted and falling back to the first
  recorded __i32 instance.
- Symptom class matches #849 (func N failed to validate, nominal
  ref-type mismatch on wasm32-gc): check whether the same name/instance
  keyed resolution underlies that S3 drift.

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
