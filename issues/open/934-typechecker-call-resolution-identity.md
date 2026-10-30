---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
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
- [ ] Two modules exporting the same function name and calling each other's
      function with different signatures type-check correctly (new fixture
      under `tests/fixtures/modules/`).
- [ ] `Vec::from_iter`-style multiple impls of one associated name are
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

