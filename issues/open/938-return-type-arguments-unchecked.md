---
Status: open
Created: 2026-10-27
Updated: 2026-10-30
ID: 938
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "#933, #935, #937, #939"
---

# 938 — return-type checking compares only the outer tag, not type arguments

## Summary

`check_return_type_compat` compares the return and value type tags (plus a
struct/enum name special case) and returns early when either side is
unknown. It never compares type arguments, so a function declared
`-> Vec<String>` may return a `Vec<i32>`.

## Evidence (2026-10-27)

- `src/compiler/typechecker/returns.ark` L7–49: tag comparison only;
  `TY_UNKNOWN` on either side returns without error; `i32`/`char` and
  struct/enum same-name pairs are accepted.

Repro (`ext_ret.ark`):

```ark
use std::host::stdio

fn strs() -> Vec<String> {
    let v = Vec::new<i32>()
    push(v, 7)
    v
}

fn main() {
    let v = strs()
    stdio::println(get_unchecked(v, 0))
}
```

Observed with `.build/selfhost/arukellt-s2-runtime.wasm`
(sha256 `1e11e8e7f2c5…`):

- `wasm32`: compiles; `wasmtime run` prints an empty line.
- `wasm32-gc`: "compiler produced invalid core Wasm".

## Expected

E0200 return type mismatch `Vec<String>` vs `Vec<i32>` on both targets.

## Acceptance

- [ ] Return values are checked with the same full, recursive type
      compatibility routine used for `let` annotations and call arguments
      (one definition of "same type" in the checker).
      (Shared with `let` annotations via `unify_compat::is_type_compatible`;
      call arguments still use `unify`, the binding form of the same identity
      knowledge, so the two implementations are not merged yet.)
- [ ] The implicit `i32`/`char` and struct/enum same-name acceptances are
      either removed or justified by an ACCEPTED ADR and covered by fixtures.
      (struct/enum is kept to mirror `unify`; `char`→`i32` is kept as an
      explicit return-only exception for `std/core/hash.ark`. Neither has an
      ADR or dedicated fixture yet.)
- [x] Compile-error fixture for the repro on `wasm32` and `wasm32-gc`.
      (`tests/fixtures/diagnostics/return_type_arg_mismatch.ark`/`.diag`;
      verified on both targets, not yet registered in `manifest.txt`.)
- [ ] `python3 scripts/manager.py selfhost fixpoint` stays green.
      (Not run — this lane was instructed not to run fixpoint.)

## Implementation (2026-10-30)

New `src/compiler/typechecker/unify_compat.ark` holds the single type-identity
predicate `is_type_compatible(env, expected, actual)`:

- resolves both sides, then compares tag, nominal name (struct/enum/resource),
  and type arguments recursively, so `Vec<String>` vs `Vec<i32>` is a mismatch;
- treats `TY_UNKNOWN` and an unbound `TY_TYPE_VAR` as compatible: neither
  proves a mismatch, and generic bodies / weakly inferred calls need the
  deferral. This keeps the old `TY_UNKNOWN` early return and stays out of
  #939's scope;
- mirrors `unify`'s unit-vs-empty-tuple and struct/enum same-nominal-name
  rules so a return check is never stricter than an argument check;
- a side with zero type arguments carries no identity to contradict, so
  `Vec::new()` accepts annotation-supplied arguments.

`returns.ark`: `check_return_type_compat` delegates to that predicate; the
tag-only comparison and the bespoke struct/enum branch are deleted.
`name_display.ark`: `type_display_name_with_args` renders full types so the
diagnostic names the differing argument.

Kept exception: `char` → `i32` in return position only (`returns_char_as_i32`).
`std/core/hash.ark`'s `impl Hash for char { fn hash(self: char) -> i32 { self } }`
depends on it and `std/**` is outside this lane's write scope.

## Verification (2026-10-30)

Isolated build (other lanes were editing `std/core/iter.ark` and
`mir/lower/core_names.ark`, so the artifact includes their in-flight state):

```
ARUKELLT_BUILD_DIR=wt/lane7/build python3 scripts/manager.py selfhost build-compiler
# passed, 56.9 s
# sha256 wt/lane7/build/selfhost/arukellt-s2.wasm =
#   4ea395da7bd6c7710955fc0ad3c3b44ed29414a00c24ca8c7ead55f598e293f5
```

Repro `wt/lane30/p938.ark` with that s2, both targets:

```
wasm32:    error[E0200|typecheck]: return type mismatch: 'strs' declared to return Vec<String> but body returns Vec<i32>
wasm32-gc: same E0200
```

Full `selfhost fixture-parity` with that s2:
`PASS=1811 FAIL=108 COMPILE_VALIDATE=1919` (run fixtures) and
`test: PASS=19 FAIL=19`. Every failure is the new E0200; none are hidden with
allowlists or skips. Classification:

- 95 × `initializer Option<i32> does not match annotation Option<String>`:
  weak inference surfaced in `std/collections/hash_string.ark:694`
  (`let _last: Option<String> = pop(items)` — the `pop` free call resolves to
  the `Vec<i32>` shape). Domains: stdlib_hashmap 73, stdlib_hashset 8,
  collections/hashmap_* 8, plus 6 singles; the 19 failing `test:` fixtures are
  the same class. Root cause is #934.
- 3 × `Vec<i32>` vs `Vec<String>`: `Vec::from_iter` overload resolution by
  name (`stdlib_trait/iterator_string`, `iterator_string_callbacks`,
  `iterator_map_type_change`). Also #934.
- 10 × fixture code errors the override used to hide:
  `let v: Vec<Vec<i32>> = Vec::new<i32>()` and equivalent
  (collections/vec_option_i32, vec_result_i32, vec_vec_i32, vec_vec_i32_get,
  vec_vec_string_get; generics_v1/nested_generic_call, nested_generics,
  nested_option; diagnostics/nested_generic; v0_constraints/no_nested_generics).
  Fixed in scope: `diagnostics/nested_generic.ark`
  (`Vec::new<Vec<i32>>()`), re-verified on both targets. The other 9 need the
  same one-line turbofish fix by their owners; after the in-scope fix the run
  total is 107 FAIL (verified only for that fixture, not re-running full
  parity).

## Lead integration (2026-10-30) — landed; final gate numbers

Two adjustments were made when merging, and the final tree is green:

1. **#937's `stmt_let.ark` half was reverted** (see #937). Its 108 parity
   failures above came from surfacing weak inference, not from this issue.
2. **Symbolic projections defer** (`typechecker/unify_compat.ark`
   `is_symbolic_projection`): an unreduced `Owner::Assoc` aggregate name is not
   proof of a mismatch, like `TY_UNKNOWN` / an unbound type var. Without this,
   the full parity run left exactly one false positive
   (`stdlib_trait/iterator_string_callbacks.ark`: declared `Vec<T::Item>`, body
   weakly inferred `Vec<i32>`). It reuses `associated_types::projection_sep_index`
   so the projection spelling has one definition.

Final verification with the merged tree (`selfhost build-compiler` PASS):

```
sha256 .build/selfhost/arukellt-s2.wasm = d8f32674dd474da3486266ae3add67faa09766cfb4035521ee6be61def165990
selfhost fixture-parity  -> PASS=1920 FAIL=0 COMPILE_VALIDATE=1920 EXECUTED=110 RUNTIME_UNSUPPORTED=2
                            test: PASS=38 FAIL=0
selfhost diag-parity     -> PASS (257 goldens; return_type_arg_mismatch registered)
repro p938 (wasm32, wasm32-gc) -> E0200 return type mismatch: 'strs' declared to return Vec<String> but body returns Vec<i32>
```

Fixture `tests/fixtures/diagnostics/return_type_arg_mismatch.ark`/`.diag` is
registered in `manifest.txt` (`diag:diagnostics/return_type_arg_mismatch.ark`).
`docs/data/project-state.toml` fixture counts were bumped by the two new
manifest entries (3550 → 3552, remainder 1962 → 1964) and docs regenerated.

Still open on this issue: acceptance 1 is shared with `let` annotations but call
arguments still use `unify` (not merged); acceptance 2 keeps the `char`→`i32`
return-only exception without an ADR/dedicated fixture; acceptance 4
(`selfhost fixpoint`) was not run.


