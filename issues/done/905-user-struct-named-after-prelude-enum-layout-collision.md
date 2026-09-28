---
Status: done
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

- [x] prelude enum と同名の user struct が enum 側ではなく自身の
      layout に解決され、wasm32-gc で valid な core Wasm を生成する。
- [x] wasm32-gc / wasm32 linear で同一の観測結果になる。
- [x] 対応しない形態は compile diagnostic で拒否する。

## Required evidence

- implementation commit SHA または採択 ADR
- positive/negative fixture の両 backend 実行結果

## Done (2026-09-28)

- root cause: `ctx.struct_names` + first-registration index が
  module qualifier を無視する basename match で、先に登録された
  prelude enum の `Box`（`_tag` 単一 i32 フィールド）を user
  `struct Box` の layout として返していた。さらに emit 層の
  `resolve_named_struct_gc_type` が last-colon strip で
  `enum:Box` → `Box` → struct ref に化ける二次経路があり、
  payload ref を i32 field に store して invalid core Wasm に
  なっていた。
- implementation: 5fa65f232 fix(#905)。struct-context の全 layout
  lookup を `ctx_struct_layout_preferred_index` 経由に統一し、
  enum base（`_tag` 1フィールド）以外の登録を優先させる。user
  struct は裸の `Box` を獲得し、prelude enum は `enum:Box` /
  `?Box:args` / `Box<args>` / `Box::Owned` spelling 経由で
  到達可能なまま残る。
- emit 側: `resolve_named_struct_gc_type` が `enum:` 名と
  head が enum variant を持つ `Head<args>` 名を struct 解決
  から除外し、`enum:` を enum_open へ戻す。`call_type_vec`
  の `enum:` rewrite は `elem_is_unshadowed_enum` で
  shadowed 名を抑制、`mark_call_result_from_return_type_name`
  と `_new` heuristic は ambiguous 名で rtn を優先する。
- generic enum 側の復元: `ctx_generic_enum_type_name` が
  `?X:args` / `X<args>` を `enum:base` に落とし、
  `mono_param_subst` は base が generic struct でなく enum の
  とき derive 前に `enum:base` を返す。
- 対応しない形態の拒否: `report_struct_enum_shadow_conflict`
  （typechecker/module_structs.ark）が、non-generic struct +
  generic enum の shadow のみ許容し、generic struct
  （`struct Box<T>`）や non-generic enum との衝突を decl 時点
  の E0101 diagnostic で拒否する。
- positive fixture: enums/prelude_enum_name_shadow.ark —
  wasm32-gc compile + wasm-tools validate PASS、wasm32 linear
  compile + run で expected の `shadow` / `42` と一致
  （close review 時に s2 5fa65f232 で再実行確認）。
- negative fixture: diagnostics/generic_struct_enum_shadow.ark
  （diag）— `struct Box<T>` が `error[E0101|resolve]:
  ambiguous type name 'Box': only a non-generic struct may
  shadow a generic enum; rename the struct or drop its type
  parameters` で拒否されることを .diag pattern で照合。
- verification: fixture-parity enums 13/13 + structs/generics/
  diagnostics 34/34 PASS、verify lane PASS、fmt PASS、
  docs regenerate + docs check 6/6 PASS（verify quick は未実行、
  lane + parity で代替）。observable-semantics rule
  `evaluation_struct_shadows_prelude_enum`（ADR-072）と
  fixture_manifest_count 3121 を efda9bdbf docs(#905) で登録。
- 残余: 同一モジュール内の `struct X` + `enum X` 衝突は従来
  通り E0101。prelude enum との shadow は non-generic struct に
  限るルールを ADR-072 系の observable semantics として記録済み。
