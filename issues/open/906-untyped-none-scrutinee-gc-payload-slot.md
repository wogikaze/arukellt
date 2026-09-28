---
Status: open
Created: 2026-09-28
Updated: 2026-09-28
ID: 906
Parent: 852
Track: language-v2
Depends on: none
Related: "#904, #902, #852"
Orchestration class: design-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 2
Source: "#904 verification residual — untyped Option::None / weak scrutinee match binds still pick the GC-ref payload slot and emit invalid core Wasm"
---

# 906 — untyped Option::None 等 payload 型情報を持たない weak scrutinee の match bind が ref slot を読み invalid core Wasm になる

## Scope

#904 により enum payload ctor は dest local に `option:<t>` /
`result:<t>:` の concrete spelling を刻むようになったが、payload を持た
ない ctor（`Option::None`）や spelling が失われた経路（`enum:Option` /
`enum:open`）は依然として弱い scrutinee 型のまま残る。その scrutinee を
`Option::Some(v)` で match すると、bind vt の推論が登録済み Some
variant の ref-field slot を選び、bind local が GC ref 宣言される一方で
arm 結果が scalar (i32) へ流れるため `expected i32, found (ref null
$type)` の invalid core Wasm になる。

- `let x = Option::None; match x { Option::Some(v) => v, _ => -1 }` —
  x は `enum:Option` のまま（None は payload 型を持たない）。
- `let x: Option<i32> = Option::None` の注釈経路は正常 — spelling が
  届く。
- pre-#902 s2（post891）および #862 s2 でも同一 failure — 長年の既存
  バグ。#904 実装の regression ではない。

## Evidence

- `none_let.ark`（`let x = Option::None`）が両旧 s2 で `type mismatch:
  expected i32, found (ref null $type)`。
- emit は `ref.cast (ref <enum-open Some slot>); struct.get 1` で `$v` を
  ref として読み、i32 result local へ `local.set` しようとする。
- weak scrut では `mir_payload_gc_field_is_ref` が同一 variant 名の
  ref-slot 登録を見て GC_REF を返し、`ctx_ensure_*_typed` は
  `_f1_ref14` を選ぶ。scalar 用途との整合は取れない。

## Design notes

- 修復方向の候補（#904 design notes の (b)/(c)）: typechecker の unify
  済み scrutinee 型を MIR local name へ伝播する、または weak scrut の
  payload bind を compile diagnostic として拒否する。slot 選択を「bind
  側が期待する vt」に合わせるには、weak scrut では情報が存在しない
  ため typechecking 層の情報が必要。
- `Option::None` ctor が `enum:Option` を刻むのは正しい（T の情報が
  存在しない）。問題は match 側が情報無しに slot を選ぶ点。

## Acceptance

- [ ] `let x = Option::None` 等の weak scrutinee に対する match が
      wasm32-gc で valid かつ正しい core Wasm を生成するか、
      対応しない形態は compile diagnostic で拒否する。
- [ ] wasm32-gc / wasm32 linear で同一の観測結果になる。

## Required evidence

- implementation commit SHA または採択 ADR
- positive/negative fixture の両 backend 実行結果
