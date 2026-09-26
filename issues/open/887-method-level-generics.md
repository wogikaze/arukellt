---
Status: open
Created: 2026-09-26
Updated: 2026-09-26
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

- [ ] Method-level type-parameter syntax と semantics を ADR/RFC で固定する
- [ ] `VecIter<T>::map` (or equivalent) が `fn(T) -> U` callback を受け、
      `MapIter<T, U>` を返す
- [ ] i32 -> String / String -> i32 item-type-changing pipeline が
      `wasm32-gc` と `wasm32` の両 backend で compile/validate/run する
- [ ] 未決定型変数が残る誤用に対する diagnostic fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
