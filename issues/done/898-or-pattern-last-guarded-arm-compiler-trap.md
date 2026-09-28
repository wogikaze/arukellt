---
Status: done
Created: 2026-09-28
Updated: 2026-09-28
ID: 898
Parent: 875
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #875, #892, #893, #852"
Orchestration class: implementation-ready
Orchestration upstream: 875
Blocks v{N}: language-v2
Priority: 1
Source: "#893 close review residual — guarded/last or-arms call undefined lower_or_match_compare_condition"
---

# 898 — or-pattern が last arm / guard 付き arm でコンパイラが trap

## Scope

legacy match lowering で or-pattern が **last arm** に来る場合
(`lower_last_pattern_condition`) と、**guard 付き arm** の
pattern condition (`lower_guarded_pattern_condition`) が、
`body_match_compare_or::lower_or_match_compare_condition` を呼ぶが
この関数は未定義。resolve が stub trap 化するため、該当形を含む
プログラムは **コンパイラ自体が wasm unreachable で異常終了** する。
中間 arm で非 guard の or-pattern (`lower_or_match_compare_prefix`)
は実装済みで動作する。

## Evidence

最小プローブ(どちらも s2 が trap):

```ark
fn main() {
    let x = 2
    match x {
        1 => stdio::println("one"),
        2 | 3 => stdio::println("two-three"),   // last arm or → trap
    }
}
```

```ark
fn main() {
    let x = 2
    match x {
        1 | 2 if x > 10 => stdio::println("big"), // guarded or → trap
        _ => stdio::println("other"),
    }
}
```

- `wasm trap: wasm `unreachable` instruction executed` at
  `mir_emit_top_level_fn_body` → `mir_emit_top_level_fn_mono_bodies`。
  wasm32-gc / wasm32 を問わず compile 時に発生。
- `body_match_compare_or.ark` (41 行) には
  `lower_or_match_compare_prefix` のみ存在し、
  `lower_or_match_compare_condition` が無い。

## Design notes

- 契約は `lower_single_match_compare_condition` と同じ:
  折りたたみ済み compare 列の末尾 bookkeeping local を返し、
  caller が `local.get(cond); if` で store policy に materialize
  させる。`lower_or_match_compare_prefix` の compare+OR-fold を
  共有 helper に括り出し、prefix 版(if→push_nesting)と
  condition 版(local を返す)の両入口にする。
- CoreHIR 経路の last/guarded arm が or-pattern を同じ意味で
  処理することも併せて確認する(`lower_core_match_last_arm` は
  non-tuple に condition を emit しない現状があり、同じ divergence
  が潜在する可能性)。

## Acceptance

- [x] last arm の or-pattern がコンパイル・実行でき、or の各
      alternative が正しく照合される。
- [x] guard 付き or-pattern arm が pattern 照合→guard 評価の
      順序で動作する。
- [x] wasm32-gc と wasm32 linear で同じ意味論・同じ観測結果になる。

## Required evidence

- implementation commit SHA
- positive fixture(or last arm / guarded or arm)の両 backend 実行結果
- 既存 `or_pattern.ark` fixture の継続 PASS

## Done (2026-09-28)

- implementation: `fix(#898)` commit.
  `body_match_compare_or.ark` の compare+OR-fold を
  `emit_or_compare_fold` へ括り出し、未定義だった
  `lower_or_match_compare_condition` を fold→local.set→local 返却の
  形で追加。prefix 版(unguarded 中間 arm)との意味論差分はない。
- positive fixture: `tests/fixtures/match_extensions/or_arm_edge.ark`
  — or last arm(`2 | 3`)、guard 失敗で catchall へ落ちる or arm、
  guard 成立で後続 alternative が発火する or arm。
  wasm32-gc component / wasm32 linear core の双方で
  `two-three / other / hit` を出力。
- 既存 `or_pattern.ark` は継続 PASS(match_extensions 5/6、
  残り 1 件は既存の `struct_pattern` 失敗)。
- 残余(別 issue): 全 arm 非マッチ時の挙動は or/literal を問わず
  非 catchall last arm 全体で既存の GC/linear divergence
  (linear=無条件実行、GC=null trap)。i32 等の literal scrutinee に
  exhaustive 診断が無い設計問題として #899 に分離。
