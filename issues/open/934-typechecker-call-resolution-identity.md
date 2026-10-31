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
- [x] The checker records the resolved FunctionId (and MonoInstanceId for
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

## Update (2026-10-31, `wave/decl-identity-final`) — entry-body declaration identity + checked call table

### Item 1 — a resolver-processed entry body resolves bare names by declaration identity

The checker resolved every body against one flat `NameIndex` built from all
loaded modules and the entry module, so a bare call bound whichever declaration
was registered first. `typechecker/entry.ark` now builds a second index for the
bodies the resolver processed:

- `bare_fn_sig_index` holds only the entry module's own top-level fn/const
  decls and prelude decls. The entry decl is inserted first, so a local
  declaration shadows a same-named prelude declaration; declaration order in
  the flat index never decides.
- `TypeEnv.entry_decl_start` records the entry module's first decl index (the
  loaded decl count); `sig_lookup::is_entry_body` gates the strict path.
- `sig_lookup::lookup_declared_fn_sig` is now the lookup used by
  `call_fn.ark` (initial name and callee-spelling variants), `call_method.ark`
  (the free/prelude fallback, `vec_method_declaration`, and the
  `get_unchecked` anchor), and `sigs.ark` (function-value resolution). In an
  entry body a bare name can no longer bind a loaded module's declaration.
- The prefix-strip fallback in `call_fn.ark` and the leaf fallback in
  `sigs.ark` are loaded-module-only now: a resolver-processed body keeps the
  whole identity the resolver accepted instead of dropping the module
  segment.

Fixture `tests/fixtures/modules/bare_decl_shadow/`: `import shapes` loads a
`measure(String) -> String` before the entry module's `measure(i32) -> i32`.
The pinned compiler binds the imported declaration and rejects the fixture with
`return type mismatch: 'shadow_probe' declared to return i32 but body returns
String`; the changed compiler accepts it and the run fixture prints
`shapes:x`. The probe is deliberately unreachable from `main` because MIR's
flat module still cannot emit a call whose callee shares a bare name with a
loaded declaration; type-checking the probe is the user-visible regression
contract, emitting it is the #933 consumption work.

Remaining item 1 blockers:

- Loaded-module bodies are still not resolver-processed, so they keep the flat
  bare-name lookup (the compiler source itself calls `infer_expr` and friends
  bare across `typechecker/`). Removing that fallback needs module-graph
  resolution of loaded bodies, not a local checker change.
- Non-conversion impl methods still share the flat `Owner::method` key
  (`entry.ark::push_sig_index_key`); disambiguating them needs a qualified
  impl identity in the signature spine (#940 owns the type side).
- MIR keys declarations by bare name in its flat module, so a call whose
  callee collides with a loaded declaration has no reachable FunctionId yet
  (see the fixture note). The checked call table below is the prerequisite for
  closing this.

### Item 2 — the checked program carries the resolved FunctionId and MonoInstanceId

`ResolvedCallOwner` becomes `ResolvedCall` with `function_id` and
`mono_instance_id`; `TypeCheckResult.resolved_calls` exposes it:

- concrete free/associated calls record `FnSig_name(selected_sig)`, including
  the conversion impl selected from the source argument;
- generic free and method calls record the declaration name and the minted
  `MonoInstance` mangled name (`push__i32`, `HashMap::insert__String__i32`);
- bound trait method calls record the resolved trait method signature;
- `len` records the owner-qualified `String::len` / `Vec::len` FunctionId.

`CoreHirResolvedCall` carries both fields, `mir/lower/ctx_resolved_calls`
indexes them beside the owner, and `ctx_resolved_call_function_id` /
`ctx_resolved_call_mono_instance_id` are the lowering accessors. `len` and
`pop` now lower from the recorded FunctionId instead of recomposing the
owner-qualified spelling (the owner remains a fallback for records without
one), so `stdlib_vec/prelude_call_owner_untracked_receiver` exercises the
FunctionId path.

Calls without a callee declaration intentionally record no FunctionId: enum
variant constructors, the compiler-provided polymorphic `clone`, and the
intrinsic fallbacks.

### Evidence

- `selfhost build-compiler`: PASS (pin `56ff0808`, s2 = `1dc9f1c856e2…`).
- `verify fixtures`: PASS=1970 FAIL=0 COMPILE_VALIDATE=1970 EXECUTED=114
  RUNTIME_UNSUPPORTED=2; `test:` PASS=43 FAIL=0.
- `verify lane`: PASS (quality changed; lint checked=21 failed=0).
- `docs check`: 6/6 PASS.
- `selfhost diag-parity`: PASS.
- `selfhost fixpoint`: s2 == s3 =
  `1dc9f1c856e20201019f61704d35e596a21f4c1d991d5f6c6641ea89ac7be67b`
  on pin
  `56ff0808171dcc3c521b0e85d07f2b16a349a59419483ad71148a5ee928d23e8`.

## Update (2026-10-31, `wave/decl-identity-loaded`) — MIR consumes the declaration-keyed callee

### Item 1 / MIR — the checker's declaration selects the emitted callee

`ResolvedCall` gained `callee_decl_index`: the selected signature's top-level
declaration in the combined loaded+entry decl list. `FnSig` carries the
declaration index it was collected from (`module_fns::collect_fn_sigs`), so a
concrete call records it without a second name lookup; CoreHIR forwards it as
`CoreHirResolvedCall.callee_decl_index`.

MIR consumes it where lowering previously keyed on the bare callee name:

- `fn_index_view::mir_build_decl_function_ids` maps each top-level decl index to
  the registry FunctionId the signature spine assigned it (the declaration
  entry only; generic mono variants and impl methods carry mangled base names
  and do not claim the slot). `ctx_resolved_calls` publishes the map on
  `LowerCtx`.
- `call_emit::mir_emit_direct_call` binds the recorded declaration's FunctionId
  per call site before `call_func_id`'s name lookup; the lookup is skipped when
  the site already carries an id, and `call_func_id` keeps an already attached
  FunctionId instead of re-binding by spelling.
- `MirFunction.decl_fid` carries the declaration id of a top-level body and
  `sections_types_sigs_detail` reads params/return through
  `spine_lookup::function_param_mvt_by_fid` /
  `function_return_mvt_by_fid`. Two modules exporting `measure` no longer share
  the first `measure` signature, so the emitted entry body has its own i32
  parameter/return type.
- Call-dest storage and print-argument conversion read the declaration's return
  contract (`ctx_decl_return`) when the flat name index would give a different
  width. Without this, the opposite collision direction (loaded scalar, entry
  String) still emitted invalid core Wasm.

### Fixture

`tests/fixtures/modules/bare_decl_shadow/main.ark` now calls `shadow_probe()`
from `main` before the qualified `shapes::measure` call; expected output is
`42` then `shapes:x`. The pre-change compiler could not emit the colliding call
(invalid core Wasm); the changed compiler compiles and runs it.

Both targets were exercised:

- wasm32-gc through the manifest `run:` fixture and
  `python3 scripts/check/check-cross-backend-parity.py --fixture
  modules/bare_decl_shadow/main.ark` (`status=parity`, `gc_rc=0`, `lm_rc=0`);
- wasm32 linear through the same cross-backend gate, plus a direct
  `compile --target wasm32 --wasi-version wasi-p1 --memory-width memory32`
  run that prints `42` / `shapes:x`.

### Remaining item 1 blockers

- Loaded-module bodies still resolve bare names through the flat signature index
  (`sig_lookup::lookup_fn_sig`). Removing that needs module-graph resolution of
  loaded bodies; the compiler source itself calls `infer_expr`-style helpers
  bare across modules, so the fallback cannot be deleted locally.
- Non-conversion impl methods still share the flat `Owner::method` key
  (`entry.ark::push_sig_index_key`); disambiguating them needs a qualified impl
  identity in the signature spine (#940 owns the type side).
- Function values and a few call-ABI decisions still consult the name index:
  `call_arg_i64`'s parameter MVT, impl-method rewriting, and `MIR_REF_FUNC`
  attachment. A collision that changes a parameter width or a function-value
  target in those positions is not covered by this increment.
- Generic calls keep the minted `MonoInstance` identity (unique) rather than the
  declaration index.

### Evidence

- `selfhost build-compiler`: PASS (pin `56ff0808`, 60.4s), s2 =
  `bbc1f2b4d5d62e5930e2e57c0628ad30f54b13f110017093ccdbaccecd2587d1`.
- `verify fixtures`: PASS=1971 FAIL=0 COMPILE_VALIDATE=1971 EXECUTED=114
  RUNTIME_UNSUPPORTED=2; `test:` PASS=43 FAIL=0.
- `verify lane`: PASS (quality changed; lint checked=36 failed=0).
- `docs check`: 6/6 PASS.
- `selfhost diag-parity`: PASS.
- `selfhost fixpoint`: s2 == s3 =
  `bbc1f2b4d5d62e5930e2e57c0628ad30f54b13f110017093ccdbaccecd2587d1`
  on pin
  `56ff0808171dcc3c521b0e85d07f2b16a349a59419483ad71148a5ee928d23e8`.
- Cross-backend receipt: `.build-lane/parity/bare_decl_shadow.json`
  (`compiler_sha256` starts `bbc1f2b4d5d62e59`, `status=parity`).

## Update (2026-10-31, `wave/loaded-body-sigs`) — loaded bodies resolve bare names in their module graph

### Item 1 — the loaded-body module-graph increment

Loaded-module bodies are still not resolver-processed, but their bare names now
have a module-graph scope instead of the flat declaration-order index.
`typechecker/loaded_body_scope.ark` builds `loaded_bare_sig_index`, keyed
`<module>::<bare name>`, from three sources in lexical precedence order:

- the module's own top-level fn/const decls;
- the symbols its `use` decls import (the member form; a leaf naming a loaded
  module is a module import and claims no callable bare name);
- the prelude decls (last, so an own declaration or explicit import shadows a
  same-named prelude free).

`TypeEnv` carries `loaded_module_names`, the index, and
`current_module_index`; `entry.ark` maps each decl index to its loaded module
(entry module is `-1`) and `sig_lookup::lookup_declared_fn_sig` routes a bare
name in a loaded body through `<module>::<name>` first. An imported symbol keeps
its defining module's exact identity: the import edge maps
`<owner>::<symbol>` from the flat index to `<importing module>::<symbol>`.

The flat lookup remains only as the fallback for a bare name the module graph
does not define. That is required by the compiler source itself, whose loaded
modules call helpers such as `infer_expr` bare across `typechecker/` without
importing them; those bodies are not resolver-processed. Qualified names and
impl-mangled names still use the exact flat keys, and a name the module graph
does define can no longer bind a same-named declaration from an earlier loaded
module.

### Fixture

`tests/fixtures/modules/loaded_bare_decl_scope/` (registered as
`module-run:` and `run:`):

- `alpha::label(i32)` and `beta::label(String)` collide by bare name; `beta`'s
  own body calls its own declaration. The flat index loads `alpha` first.
- `gamma` imports `delta::measure(i32)` while `alpha::measure(String)` holds the
  flat bare slot; `gamma`'s bare `measure(2)` must bind the import.

The pinned compiler rejects both bodies (`E0200` for `label` and `measure`, plus
the wrong `probe_beta` / `probe_gamma` return contracts). The changed compiler
accepts them and the run fixture prints `42`, `beta!`, `12`. The fixture also
documents the remaining resolver gap: the pinned and changed compilers both
report `W0006` unused import `delta::measure`, because loaded-module bodies are
not resolver-processed.

### Evidence

- `selfhost build-compiler`: PASS, 66.4s, s2 =
  `5c6aadaad0376af304e040f253083f52c3cc98c0631e43fe8832ba1ee438461c`.
- `verify fixtures`: PASS=1980 FAIL=0 COMPILE_VALIDATE=1980 EXECUTED=115
  RUNTIME_UNSUPPORTED=2; `test:` PASS=43 FAIL=0.
- `verify lane`: PASS (quality changed; lint checked=13 failed=0).
- `docs check`: 6/6 PASS after regenerating the fixture-registry count
  (`3701` → `3703`; `fixture_not_in_last_harness_snapshot` `2113` → `2115`).
- `selfhost diag-parity`: PASS.
- `selfhost fixpoint`: s2 == s3 =
  `5c6aadaad0376af304e040f253083f52c3cc98c0631e43fe8832ba1ee438461c`
  on pin
  `bc81d60b338480bce5b45f185652e409a4af4235b34a655b0ebeac5411734aff`.

### Remaining item 1 blockers

- The flat fallback is still reachable for a loaded-body bare name that the
  module graph does not define (the compiler source's bare cross-module helper
  calls). Closing it needs resolver processing of loaded bodies — imports and
  accepted identities on those ASTs — not another local lookup change.
- Import resolution follows direct `use owner::symbol` edges. A symbol imported
  from a module that only re-exports it (`pub use source::symbol`) has no
  signature key under the facade module, so it still falls back to the bare flat
  lookup. Resolving `pub use` chains in the typechecker module graph is the next
  increment.
- The qualified-name fallbacks (`call_fn.ark` prefix strip, `sigs.ark` leaf) and
  the flat `Owner::method` impl key are unchanged; they are name-derived, not
  module-graph-derived, and are tracked by the existing residual bullets above
  and #940.
- `reg-vt-audit` reports return-VT mismatches for the fixture's colliding callee
  names (`label`, `probe_beta`) because the emitter's legacy name-keyed return
  lookup still sees the first same-named declaration. The recorded declaration
  drives the emitted call and the runtime output is correct; the audit path is
  not a gate.
