---
Status: open
Created: 2026-10-27
Updated: 2026-10-31
ID: 934
Track: compiler
Depends on: none
Orchestration class: architecture-implementation
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-040, ADR-042, ADR-046, #933, #935, #936"
---

# 934 — typechecker resolves calls through a flat, overload-blind name index

## Summary

Function signatures are looked up by flat (module-stripped) name in
`src/compiler/typechecker/sig_lookup.ark`. The index cannot distinguish
same-named functions in different modules or stdlib bodies that share short
names. Because of this, concrete calls skip argument checking (#935), and
the typechecker never produces a resolved callee identity that lowering
could consume (#933).

## Evidence (2026-10-27)

- `src/compiler/typechecker/call_fn.ark` L153–157 states the reason for not
  unifying arguments: "flat name lookup is not overload-aware and would
  false-fail stdlib bodies that share short names."
- `src/compiler/typechecker/call_fn.ark` `tc_strip_module_prefix` and
  `sig_lookup.ark` key signatures by bare name.
- `src/compiler/typechecker/stmt_let.ark` L48–52 documents a second symptom:
  "`Vec::from_iter` picking the first impl by name" produces
  concrete-but-wrong argument types, so annotations are allowed to override
  inferred types (#937).

## Expected

Every call / method call / path call is resolved in the checker to one
declaration identity (module path + item + impl), which carries its exact
signature. Ambiguity is a diagnostic, never "first match by name".

## Acceptance

- [ ] Signature lookup is keyed by module-qualified symbol identity (and impl
      for methods / associated functions); no bare-name fallback.
- [ ] The checker records the resolved FunctionId (and MonoInstanceId for
      generic calls) per call node and exposes it in `TypeCheckResult` for
      #933.
- [x] Two modules exporting the same function name and calling each other's
      function with different signatures type-check correctly (new fixture
      under `tests/fixtures/modules/`).
- [x] `Vec::from_iter`-style multiple impls of one associated name are
      selected by type, not declaration order (fixture).
- [ ] `python3 scripts/manager.py selfhost fixpoint` stays green.

## Update (2026-10-30, `wave/decl-identity`) — HashMap shells bind the receiver pair

The concrete failure behind the `hashmap_methods.ark` pre-emption (recorded by
b2dc1d4c5) is fixed at the declaration-identity layer.

Root cause, two layers:

- `impl<K, V> HashMap<K, V>` methods entered the signature table with an
  untyped `self` (`module_fns::impl_self_param_sig`). The checker names a
  generic method's mono instance from the unified self parameter, so
  `m.insert(k, v)` recorded `insert__String__i32`: the `HashMap` owner never
  entered the callee identity, and `get` could not instantiate `V` from the
  receiver at all. That is why removing the pre-emption lowered the call
  through the bare name.
- MIR then inferred set-ness from the receiver storage alias. `hashmap:si32`
  is both `HashMap<String, i32>` and `HashSet<String>`, so the HashSet
  fallback rewrote the map's `insert` / `remove` to `hashset_str_insert` /
  `hashset_str_remove` (bool instead of `Option<V>`).

The fix:

- `typechecker/module_fns.ark`: the generic `HashMap<K, V>` shell now binds
  `self` to `HashMap<K, V>` (Vec keeps its untyped adapter self). K/V
  instantiate from the receiver, the resolved callee is
  `HashMap::insert__String__i32`, and `get` returns `Option<V>` from the
  declaration instead of a name table.
- `typechecker/hashmap_methods.ark`: `infer_builtin_hashmap_method`, its
  arity table and its return-type table are deleted. The constructor entry
  point stays: `HashMap::new` / `with_capacity` carry their pair as
  turbofish arguments, not through a receiver.
- `mir/lower/call_rewrite_hashset.ark`: a call is a set call only when the
  receiver is marked (constructor / store / parameter / method self) or the
  callee carries a `HashSet` spelling. The receiver-type fallback for
  `insert` / `contains` / `remove` is gone; that alias is shared with plain
  maps and vecs.
- `mir/lower/core_bindings.ark`: a CoreHIR `let` copies the hashset marker
  from its initializer, so `let s = HashSet::new<String>()` keeps the set
  evidence in test bodies.

Fixture: `tests/fixtures/stdlib_hashmap/insert_owner_identity.ark` declares
three `insert` owners (HashMap<String, i32>, HashSet<String>, user
`Accumulator`) in one test and asserts the Option / bool / i32 contracts.

Evidence (s2 = `815047887860e3d4…`, pin `56ff0808…`):

- `selfhost build-compiler`: PASS, 59.0s.
- `verify fixtures`: PASS=1945 FAIL=0 COMPILE_VALIDATE=1945 EXECUTED=112
  RUNTIME_UNSUPPORTED=2; `test:` PASS=43 FAIL=0.
- `verify lane`, `docs check` 6/6, `selfhost diag-parity` pass.
- `selfhost fixpoint`: s2 == s3 =
  `815047887860e3d411087f664067c874255a518ae94a586f8175b40e883bb7d9`.

Acceptance movement: items 1 and 2 move for HashMap methods — the resolved
callee keeps the owner-qualified declaration identity, and the checker
records `HashMap::<method>__<K>__<V>` mono sites instead of bare names. The
general bare-name fallback in `call_method.ark` / `sigs.ark`, the
module-qualified key for every symbol, and the Vec shell's untyped self are
unchanged. Items 3 and 4 have no change.

## Update (2026-10-31, `wave/decl-identity-rest`) — items 3/4

### Item 3 — conversion impls are selected by declaration identity

The signature index no longer aliases a conversion impl's family spelling
(`Vec_String::from_iter`) to whichever impl happens to be registered first.
An impl method whose mangled name carries a conversion source
(`Vec_String::from_iter__VecIter_String`) now gets no `Owner::method` index key
from `typechecker/entry.ark`; its only identity is the mangled name.

`typechecker/call_fn.ark` resolves the source call instead: when
`Owner::method` does not name a declaration, `conversion_impl_candidate`
infers the first argument's type, builds the declared identity
`Owner::method__<Source>`, and selects the unique signature whose owner family
matches (`Vec` / `Vec_String`, `HashSet_i32`, `HashMap_String_i32`, ...). The
call is then typed and arity-checked from that declaration. This replaces the
two hardcoded family probes (`hashset_from_iter_candidate`,
`hashmap_from_iter_candidate`) with one declaration-driven rule for the
concrete conversion families (Vec, HashSet, HashMap) and the `from_iter`,
`try_from`, `from` method names their impls use.

Before: `let xs = Vec::from_iter(<Vec<String> iterator>)` bound the first Vec
impl (`Vec::from_iter__VecIter_i32`) and typed `xs` as `Vec<i32>`; a `let`
annotation then silently overrode the wrong element type. After: the call
binds `Vec_String::from_iter__VecIter_String` (or the `MapIter_String_String`
/ `TakeIter_...` / `Skip...` impl matching the source) and returns the
declared `Vec<String>` type.

Fixture: `tests/fixtures/stdlib_trait/vec_from_iter_impl_selection.ark`
covers i32, String and mapped-String sources, each read back through
`get_unchecked` with a typed `let`. Pre-fix it fails with
`E0200 type mismatch: i32 vs String`; post-fix it compiles and runs. A probe
with `<Vec<String> iterator> -> let n: i32` now reports `String vs i32`
instead of the old `Vec vs i32`.

### Item 4 — module-same-name fixture

`tests/fixtures/modules/same_name_export/` exports `describe` from two
modules: `counter::describe(i32) -> i32` and `labeler::describe(String) ->
String`. `labeler::describe_count` calls `counter::describe` across the module
boundary, and `main` binds each result to its declared type before printing
`42`, `ready!`, `10-ok`. The fixture is registered as `module-run:` and
`run:` in the manifest.

Module-qualified keying already resolved these calls before this change (the
fixture also passes on the pinned compiler), so item 4 needed the fixture, not
new resolution code. The fixture fails if either qualified call binds the
other module's declaration: `let label: String` would receive `i32`.

### Remaining

Item 1 stays partial: only the conversion-impl alias is gone. The general
bare-name fallback in `call_method.ark` / `sigs.ark` and the flat
`Owner::method` key for non-conversion impls are unchanged.

Item 2 is unchanged: the conversion call is typed from its declaration, but
the checker does not yet record a resolved FunctionId / MonoInstanceId per
call node in `TypeCheckResult`.

### Evidence

- `selfhost build-compiler`: PASS, 55.7s (pin `56ff0808`).
- `verify fixtures`: PASS=1967 FAIL=0 COMPILE_VALIDATE=1967 EXECUTED=113
  RUNTIME_UNSUPPORTED=2; `test:` PASS=43 FAIL=0.
- `verify lane` (quality changed): PASS. `docs check`: 6/6 PASS.
- `selfhost diag-parity`: PASS.
- `selfhost fixpoint`: s2 == s3 =
  `e6e4f49a5157fce286485bc6c3ca0476270bd5b332df95356a3eafa43ef0c42e`
  on pin
  `56ff0808171dcc3c521b0e85d07f2b16a349a59419483ad71148a5ee928d23e8`.
- Isolated-worktree note: the lint step of `verify lane` resolves the compiler
  from `<root>/.build/selfhost` and ignores `ARUKELLT_BUILD_DIR`, so this lane
  ran it with
  `ARUKELLT_SELFHOST_WASM=<build-dir>/selfhost/arukellt-s2-runtime.wasm`
  (the documented override). Without it the step lints with the pinned
  bootstrap and reports the pre-fix fixture error.
