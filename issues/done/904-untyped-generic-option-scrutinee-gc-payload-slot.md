Status: done
Created: 2026-09-28
Updated: 2026-09-28
ID: 904
Parent: 875
Track: language-v2
Depends on: none
Related: "ADR-058, ADR-082, #893, #901, #902, #852"
Orchestration class: design-ready
Orchestration upstream: 875
Blocks v{N}: language-v2
Priority: 2
Source: "#902 verification residual — untyped/weakly-spelled generic Option scrutinees read the GC-ref payload slot and emit invalid core Wasm"
---

# 904 — generic Option scrutinee の型 spelling が無いと wasm32-gc が payload を ref slot で読み invalid core Wasm になる

## Scope

wasm32-gc では GC enum variant が payload 値型ごとに別 struct slot を持つ
(i32 payload の Option::Some は type 51, ref payload は type 56 のように)。
match の payload 読み出しは scrutinee local の型 spelling
("Option_i32" / "option:i32" 等) から payload vt を推論して slot を選ぶ。
scrutinee が次の形だと spelling が "enum:open" / "enum:Option" 等に弱まり、
payload を常に generic ref slot で読むため、ctor が作った scalar/tuple
slot と不整合な struct.get を emit して invalid core Wasm になる。

- `match Option::Some(7) { Option::Some(v) => v, _ => 0 - 1 }`
  (inline ctor scrutinee)
- `let x = Option::Some(7)` 後の `match x` (推論 let 束縛)
- `let x: Option<(i32, i32)> = Option::Some((2, 9))` など、
  注釈はあっても payload machinery の読む spelling に落ちない場合
  (tuple 要素抽出が enum 基底 struct の field を読み field OOB)

`let x: Option<i32>` の注釈や `fn mk() -> Option<i32>` の戻り値経由では
spelling が届くため正常。wasm32 linear は全経路正常。

## Evidence

- `m_bind_let.ark` / `m_bind_inline.ark` は #902 適用前の s2
  (arukellt-s2-runtime-post891.wasm) でも
  `type mismatch: expected i32, found (ref null $type)` で invalid —
  既存バグであり #902 の regression ではない。
- `let x: Option<i32> = Option::Some(7)` (m_bind_annotated.ark) は valid。
- `let x: Option<(i32, i32)>` (a_tupor.ark) は
  `unknown field: field index out of bounds` — payload temp が
  enum 基底 struct (tag field のみ) として読まれ、tuple 要素 index が
  存在しない。
- linear (wasm32, wasi-p1) は全形態で正しい結果。

## Design notes

- ctor `lower_payload_variant_from_locals` は dest local に
  `enum:<EnumName>` のみを刻み、型引数 (Option_i32 / option:i32) を
  落とす。stmt_let_store は init_type が option:/result:/Option_/Result_
  のときだけ enum:Option を置き換えるため、ctor 由来の enum:Option は
  そのまま残る。
- 修復方向の候補: (a) enum ctor が payload arg の vt/型名から
  monomorphic spelling を dest local に刻む、(b) payload vt 推論が
  declared field type の型パラメータを登録済み mono インスタンス経由で
  解決する、(c) match payload machinery が scrutinee value の GC variant
  slot から逆引きする。(a) が ctor 一点修正で最も影響面が小さいと思われる。
- `Option::Some(v) => v` の plain bind も同じ穴を共有するため、
  #902 の sub-pattern 条件だけでなく named-bind 経路全般が対象。

## Acceptance

- [x] inline ctor / 推論 let / Option<tuple> 注釈の scrutinee に対する
      match が wasm32-gc で valid かつ正しい core Wasm を生成する。
- [x] wasm32-gc / wasm32 linear で同一の観測結果になる。
- [x] 対応しない形態は compile diagnostic で拒否する — 本 scope の
      3 形態は全て対応済みのため silent invalid は残らない。検証で
      発見した別機構の未対応形態 (payload 型情報を持たない
      Option::None scrutinee、prelude enum 同名 user struct) は
      現状 silent invalid のまま #906 / #905 として open 追跡する。

## Required evidence

- implementation commit SHA または採択 ADR
- positive/negative fixture の両 backend 実行結果

## Done (2026-09-28)

- root cause: enum payload ctor (lower_gc_payload_variant_from_locals)
  が dest local に enum:Option / enum:Result のみを刻み、match
  payload read が weak scrutinee の generic ref slot を選択した。
  ctor が書いた scalar/tuple slot と不整合な struct.get を emit
  して invalid core Wasm になっていた。
- implementation: 97d82a765 fix(#904)。gc_ctor_enum_local_type が
  storage hint (typed_name) から option:<t> / result:<t>: を導出
  して dest local に stamp する。hint が enum:open のときは
  payload local の semantic 型へ fallback
  (gc_ctor_semantic_payload_segment) し、Some(Some(v)) のような
  nested enum payload の要素型も保持する。Result::Err は Ok
  segment を供給できないため enum:Result のまま、非 generic
  enum は declared field type が payload vt を持つため
  enum:<name> のまま。
- type_ann_local_name.ark: Option/Result の tuple payload 注釈が
  tuple element spelling (Option_tuple:a,b 等) を解決し、
  generic ref slot への collapse を防ぐ。
- positive fixture: tests/fixtures/match_extensions/
  untyped_option_scrutinee.ark (run:/t3-compile:/t3-run:) →
  7 seven 7 7 8 11 hi 9 -1 7 9 7 -1 を wasm32-gc component と
  wasm32 linear core で expected と同一出力。inline ctor、
  inferred let、nested Option、tuple annotation、untyped Result、
  fn-arg/return 経由の 13 ケースに加え、31 probes が両 backend
  で同一結果。
- 継続 PASS: check-t3-wasm-validate で新 fixture は PASS
  (fail 46 件は trait/i64/diagnostics 系の既存 backlog —
  pre-#902 s2 で同一 failure を確認、regression ではない)。
  verify fixtures (fixture-parity, wasm32-gc+wasi-p2) は当該
  fixture PASS、ambient FAIL=259 は stdlib backlog 領域のみで
  match/enum 系の新規 fail なし。verify lane PASS、fmt --check
  PASS、docs check 6/6 PASS、observable-semantics gate PASS
  (match_untyped_option_result_scrutinee, ADR-060/ADR-082)。
- 残余(別 issue): payload 型情報がどこにも存在しない weak
  scrutinee (let x = Option::None 後の Some(v) bind) は依然
  invalid core Wasm — typechecker の unified 型伝播か診断拒否が
  必要で #906 へ分離。prelude enum 同名 user struct (Box) の
  layout 名衝突は #905 へ分離。
