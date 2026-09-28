---
Status: done
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

- [x] `let x = Option::None` 等の weak scrutinee に対する match が
      wasm32-gc で valid かつ正しい core Wasm を生成するか、
      対応しない形態は compile diagnostic で拒否する。
- [x] wasm32-gc / wasm32 linear で同一の観測結果になる。

## Required evidence

- implementation commit SHA または採択 ADR
- positive/negative fixture の両 backend 実行結果

## Done (2026-09-28)

- root cause: weak scrutinee (enum:Option / enum:Result / enum:open /
  untyped local) には payload 型 segment が存在しないのに、
  match payload bind の GC slot 選択が同名 variant の登録順
  guess に依存していた。i32 / i64 / f64 / ref payload が同一
  variant 名で混在できるため、guess は原理的に信頼できず、外れ
  ると runtime value が持たない struct.get を emit して invalid
  core Wasm になっていた。emit 層では bind の使用 context を
  知る手段がなく、typechecker の unified 型は MIR 到達前に
  release されるため、MIR 層での完全精度修復は不可能 —
  issue design notes の選択肢通り compile diagnostic 化を採用。
- implementation: 88a99848c fix(#906)。
  mir_report_weak_payload_scrutinee
  (src/compiler/mir/lower/match_payload_field_type.ark) が、
  payload hint 空 + wasm32-gc + Some/Ok/Err + named ident bind の
  組合せで MirModule へ diagnostic を記録する。wildcard /
  literal / composite sub-pattern は pattern 自体が payload の
  証明になるため対象外。generic template 本体
  (MirFunction_has_unsubstituted_generic_param または mangled
  name の `?` 含有) は emit 対象でなく monomorphized instance
  側が option:<t> spelling を持つため skip — template で発火
  させると generic adapter を import する全 program を誤拒否
  する。pipeline_backend が mir_opt 後に
  MirModule_drain_diagnostics で回収し PHASE_LOWER の
  CompileResult_err として返す。
- propagation fix: is_option_i32_hash_query に
  hash_string::hashmap_str_get / hashmap_str_remove を追加し、
  stdlib adapter 内部の match が option:i32 spelling を保持して
  rescue されるようにした (#904 と同型)。
- positive fixture: untyped_option_scrutinee.ark — wasm32-gc
  component compile + validate PASS、wasm32 linear run で
  expected の 13 行 (7 seven 7 7 8 11 hi 9 -1 7 9 7 -1) と一致。
- negative fixture: untyped_none_scrutinee_bind.ark
  (compile-error) — wasm32-gc で
  "unsupported match on an untyped Option/Result scrutinee:
  cannot determine the payload type for `Option::Some` —
  annotate the scrutinee with a concrete type" diagnostic。
  wasm32 linear は payload read が型選択を要しないため従来
  通り compile + run して `none` を出力 — issue の受理条件
  「対応しない形態は diagnostic で拒否」に従う設計上の
  divergence で、linear 側は常に正しく動く。
- acceptance (b)「同一の観測結果」は support 対象の program
  について確認: 正例 fixture が GC compile/validate と linear
  run で同一出力、fixture-parity 1522 PASS の fail 名集合が
  pre-#906 baseline と完全一致 (diagnostic が発火するよう
  になったのは baseline で既に validate-fail だった 2 件のみ、
  silent invalid から診断への改善)。T3 validate
  474p/16vf/30cf も baseline と完全一致で over-fire なし。
- verification: python3 scripts/manager.py verify lane PASS、
  fmt PASS、docs regenerate + docs check 6/6 PASS。
- 残余 (別 issue): prelude enum 同名 user struct (Box) の
  layout 衝突は #905 で追跡継続。diagnostic で拒否される形態
  を将来 typechecker の unified 型伝播で救済する余地は残る
  が、issue の受理条件 (a) の「または diagnostic で拒否」
  側で完了とする。
