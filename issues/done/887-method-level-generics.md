---
Status: done
Created: 2026-09-26
Updated: 2026-09-27
ID: 887
Parent: 852
Track: language-v2
Depends on: "856, 860"
Related: "ADR-058, RFC-011, #852, #860"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 2
Source: "#860 RFC-011 item 37 residual — FilterIter<T>::map T -> U"
---

# 887 — Method-level generics (`fn m<U>` on generic/非 generic types)

## Scope

RFC-011 item 37 residual transferred from #860: allow a method (or
associated function) to declare its own type parameters beyond the
impl/struct type parameters, so a callback can change the item type.

Today every type variable in an `impl VecIter<T>` method signature is
bound by the impl header, so `fn map(self: VecIter<T>, f: fn(T) -> U)`
cannot exist — `U` is unbound. The concrete consequence in stdlib:
`FilterIter<T>::map` / `VecIter<T>::map` stay `T -> T` and the
item-type-changing pipeline is only reachable by spelling
`MapIter<T, U>` construction explicitly.

This is a compiler pipeline feature, not an iterator-surface gap:
method/function type-parameter syntax (`fn map<U>(self, f: fn(T) -> U)`
or an equivalent spelling fixed by ADR/RFC), resolver binding of the
per-call type variable, typecheck unification against the call-site
argument, and monomorphization of per-instantiation method bodies on
both `wasm32-gc` and linear `wasm32`.

## Acceptance

- [x] Method-level type-parameter syntax と semantics を ADR/RFC で固定する
- [x] `VecIter<T>::map` (or equivalent) が `fn(T) -> U` callback を受け、
      `MapIter<T, U>` を返す
- [x] i32 -> String / String -> i32 item-type-changing pipeline が
      `wasm32-gc` と `wasm32` の両 backend で compile/validate/run する
- [x] 未決定型変数が残る誤用に対する diagnostic fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する

## Progress receipt — 2026-09-27 (issue closing)

- Implementation: `b11f9bdbd` (method generics inference + strict
  unification + stdlib migration), docs sync `8d2e8867a`.
- Normative spec: ADR-079 (ACCEPTED) —
  `docs/adr/ADR-079-language-v2-method-level-generics.md`; RFC-011
  item 37 row updated to reference it.
- Mechanism: `constrain_generic_method_args` recovers the declared
  signature of bare fn-name callback args (same recovery as #860 free
  fns); `unify` resolves bound vars instead of silently rebinding, so a
  callback disagreeing with an earlier binding is a typecheck error;
  concrete-receiver calls that leave method-declared params unresolved
  emit `cannot infer type parameter U in method call name` instead of
  dropping the mono instance. impl params on untyped-self receivers
  (Vec/HashMap adapters) keep the deferred path.
- stdlib: `VecIter<T>::map<U>(f: fn(T) -> U) -> MapIter<T, U>` and
  `FilterIter<T>::map<U> -> FilterMapIter<T, U>` (struct widened to two
  params; IntoIterator / Vec<String> FromIterator impls and the MIR
  Vec-String from_iter name list updated).
- Positive fixture: `tests/fixtures/stdlib_trait/iterator_map_type_change.ark`
  (run + t3-compile + t3-run). LM core module validates and runs
  `n7 n42 2 4 4 cdef`; wasm32-gc component validates and runs identically.
- Diagnostic fixture: `tests/fixtures/diagnostics/method_generic_unresolved_var.ark`
  + `.diag` — `check` emits `error[E0200|typecheck]: cannot infer type
  parameter U in method call pick`.
- Gates: `verify lane` PASS; T3 gate 478 pass / 27 compile-fail / 19 skip
  vs HEAD baseline 477/27/19 — the new fixture adds the +1 pass, the 27
  compile-fails are identical to HEAD (pre-existing: i64 literal
  annotations, enum/supertrait/member resolution, io/host capability,
  intentional negatives). `selfhost diag-parity` 61 pass / 1 pre-existing
  FAIL (`module_import/use_symbol_not_found` expects retired E0501 while
  resolve emits E0100 — unrelated golden drift).
- Residuals (tracked in ADR-079 再検討条件, not TODOs): turbofish-style
  explicit method type args; trait-declared method type params; strict
  unification of enclosing-function declared vars.
