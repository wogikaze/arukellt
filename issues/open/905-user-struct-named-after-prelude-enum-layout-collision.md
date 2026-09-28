---
Status: open
Created: 2026-09-28
Updated: 2026-09-28
ID: 905
Parent: 852
Track: language-v2
Depends on: none
Related: "#904, #852"
Orchestration class: design-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 2
Source: "#904 verification residual — user struct named after a prelude enum (Box) resolves to the enum layout and emits invalid core Wasm on wasm32-gc"
---

# 905 — prelude enum と同名の user struct が enum 側の layout に解決され wasm32-gc で invalid core Wasm になる

## Scope

`struct Box { v: Option<i32> }` や `struct Box { s: String }` のように、
prelude の `enum prelude::Box`（Box::Owned）と同名の user struct を定義
すると、layout registry (`ctx.struct_names` + `struct_layout_first_index`)
は先に登録された enum 側の `Box` に名前解決する。結果として GC struct
signature は enum の `_tag` 単一 i32 フィールドで構築され、ref payload を
持つ user struct 値の `struct.set` / `struct.get` が `expected i32, found
(ref null $type)` で invalid core Wasm になる。

- `struct Box { v: Option<i32> }; let b = Box { v: Option::Some(7) }` で
  `struct.set` が invalid（field type が ref ではなく i32 に確定）。
- `struct Box { s: String }; b.s` でも同じ failure — Option 固有ではなく
  ref payload フィールド全般の名衝突。
- `Boxx` など prelude enum と衝突しない名前では正常。
- wasm32 linear は正常（linear は byte layout で GC struct sig を使わない）。
- pre-#902 s2（arukellt-s2-runtime-post891.wasm）でも同一 failure —
  長年の既存バグであり #904 実装の regression ではない。

## Evidence

- HIR dump は `enum prelude::Box`（variant `Box::Owned`）と `struct Box`
  の双方を列挙する。`ctx_find_layout_name` / `ctx_count_layout_matches`
  は exact match が無い名前に対し basename scan で先に登録された
  `prelude::Box`（enum、`_tag` 単一フィールド）を返すため、user decl が
  修飾名で登録されると `Box` への参照が enum layout に落ちる。
- emit された GC struct は `(struct (field (mut i32)))` の 1 フィールド
  shell で、payload の ref を `_f0_i32` に store しようとして invalid。
- `.build/p902/dwn_struct.ark`（Option field）と `dwn_struct_str.ark`
  （String field）で再現。`dwn_struct2.ark`（`Boxx` rename）は valid。

## Design notes

- `name_index_insert_first` が first registration を返すため、prelude
  enum と user struct が同名の場合、user decl が全く参照されない。
- 修復方向の候補: (a) user struct decl が同名 prelude enum 名を shadow
  する（registry key を module qualifier で分離、または user decl を
  last-wins にする）、(b) typechecker が prelude enum と同名の struct
  decl を diagnostic で拒否する。

## Acceptance

- [ ] prelude enum と同名の user struct が enum 側ではなく自身の
      layout に解決され、wasm32-gc で valid な core Wasm を生成する。
- [ ] wasm32-gc / wasm32 linear で同一の観測結果になる。
- [ ] 対応しない形態は compile diagnostic で拒否する。

## Required evidence

- implementation commit SHA または採択 ADR
- positive/negative fixture の両 backend 実行結果
