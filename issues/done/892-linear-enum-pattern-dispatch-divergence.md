---
Status: done
Created: 2026-09-27
Updated: 2026-09-27
ID: 892
Parent: 875
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #875, #852"
Orchestration class: implementation-ready
Orchestration upstream: 875
Blocks v{N}: language-v2
Priority: 1
Source: "#875 cross-backend parity audit — first observed divergence"
---

# 892 — wasm32 linear の enum pattern dispatch が wasm32-gc と diverge する

## Scope

#875 の cross-backend parity audit で観測された実 divergence。
multi-clause 関数の引数パターンおよび tuple 内 enum パターンが
wasm32 (linear memory) バックエンドで壊れる。wasm32-gc では正しく
動作するため、意味論 parity の acceptance を阻害する。

この issue の独立範囲だけを実装する。別の設計判断が必要なら
child issue を追加し、ここへ混ぜない。

## Evidence

最小プローブ(両 backend で同一ソース):

```ark
enum Opt { Some(i32), None }
fn describe(Opt::Some(v): Opt) -> String | v > 10 = "big"
fn describe(Opt::Some(_): Opt) -> String = "small"
fn describe(Opt::None: Opt) -> String = "none"
```

- wasm32-gc: `describe(Opt::None)` → `none`(正しい)
- wasm32: `describe(Opt::Some(42))`→`big`, `Some(5)`→`small`
  までは出力するが `describe(Opt::None)` で
  `memory fault at wasm address 0xfffffffc` → rc=134。
  仮説: payload 無しバリアントは linear で unboxed tag なのに、
  clause 分岐が record pointer として tag field を読んでいる。

第二プローブ(tuple 内 enum パターン):

```ark
let t = (o, 7)
match t {
    (Opt::Some(v), _) => "some",
    (Opt::None, _) => "none",
}
```

- wasm32: **compile 時点**で invalid core wasm
  (`type mismatch: expected i32 but nothing on stack`)。
- wasm32-gc: 未検証(要確認 — component emit で同様に壊れる場合は
  共通の match lowering 問題)。

## Root cause(2026-09-27 確定)

二つの独立した lowering 欠陥が重なっていた。両方とも CoreHIR 経路
(`lower_core_match`) のみ、または両経路共通で、GC 側は fn-clause が
legacy body 経路へ落ちるため表面化していなかった。

1. guarded non-catchall arm の連鎖構造
   (`core_match_arm_branch.ark`):
   `if(pat_cmp) { binds; if(guard) { body } else { 次arm } }` と
   生成し、pattern-compare if に else が付かなかった。pattern miss で
   match result が未初期化のまま返り、linear では println の null deref
   (`0xfffffffc`) として顕在化した。legacy body 経路の matched-local
   方式(`lower_guarded_nonlast_match_arm`)に揃えて修正。
   同時に or-pattern + guard の arm が guard 自体を silent drop
   していた欠陥も同経路で解消。

2. payload-less variant の compare_idx
   (`mir_prepare_match_payload` / `core_prepare_match_payload`):
   linear では `is_payload_match`(子を持つ PatVariant がある)時だけ
   compare_idx を tag local にしていた。`match o { Opt::None => _, _ => _ }`
   のように payload-less arm のみの match では record pointer と tag
   定数を `eq` 比較し常に false となり wrong-arm を返していた。
   payload-enum の全 variant(record `{_tag}` 表現)を
   `ctx_is_payload_variant` 登録で判定し、tag local へ promote するよう
   両経路で修正。

tuple arm パターンは両 backend・両経路で未実装(共通 gap、divergence
ではない)のため #893 へ分離。

同一意味を `match o { Opt::None => ... }`(非 tuple)で書くと
wasm32 でも正しく `none` を返す。つまり壊れるのは multi-clause
desugar が生成する tuple scrutinee 経路と、明示的な tuple 内 enum
パターンの両方である。

## Acceptance

- [x] 第一プローブ(enum pattern dispatch)が wasm32 で
      compile/validate/run し、wasm32-gc と stdout が一致する。
      第二プローブ(tuple arm)は両 backend で同一に未実装のため
      divergence ではなく共通 gap と確定し、実装 acceptance は
      #893 へ移管した。
      wasm32-gc と stdout が一致する。
- [x] 既存の `match_extensions` / multi-clause fixture が両
      backend で回帰なし。
- [x] diverge の root cause(pattern 分岐が payload-less variant の
      storage 形を区別していない、等)がコードコメントまたは
      issue 内に記録される。

## Required evidence

- implementation commit SHA: `3cef5f4f1`
- positive fixture: `tests/fixtures/enums/match_payload_enum_unit_arm.ark`
  (`run:` / `t3-compile:` / `t3-run:` 登録)。tuple arm 相当の
  positive fixture は #893 の acceptance。
- 両 backend の compile/validate/runtime 出力:
  `.build/probes/bx/clause.{w32.wasm,wasm32-gc.wasm}` → 両者 `none`;
  `.build/probes/bx/c1.{w32.wasm,wasm32-gc.wasm}` → 両者 `none`;
  parity receipt `.build/parity/report-892.json`
  (20 fixtures, parity=19, compile-fail:gc=1 pre-existing E0200,
  diverge=0)。

## False-done guard

- [x] Acceptance の一部だけで done にしない — 第一プローブは完全
      修復、第二プローブは共通 gap として #893 へ open 残置し、
      divergence としての完了主張を残していない。
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition
      を必須にする — #893 が tuple arm の tracker。
- [x] 親 #875 の parity 表を evidence 付きで更新する —
      `.build/parity/report-892.json` を receipt として記録。
