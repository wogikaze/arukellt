---
Status: done
Created: 2026-09-27
Updated: 2026-09-28
ID: 891
Parent: 872
Track: language-v2
Depends on: none
Related: "ADR-058, ADR-061, RFC-011, #872, #852"
Orchestration class: design-ready
Orchestration upstream: 872
Blocks v{N}: language-v2
Priority: 2
Source: "#872 residual — mutable-borrow capture escape E0213 fallback"
---

# 891 — mutable-borrow capture escape の env cell 昇格

## Scope

ADR-061 により、non-move closure が capture source の place を rebind
する場合、その capture は mutable borrow となり、escape 境界へ到達した
時点で E0213 を発行する(honest fallback)。環境が source mutation を
保持できるなら mutable borrow も escape できるのが設計意図であり、
それには **capture source 側の local を managed cell に昇格** する
必要がある。

この issue の独立範囲だけを実装する。別の設計判断が必要なら
child issue を追加し、ここへ混ぜない。

## 背景

- move/Copy capture の env cell は実装済み
  (`tests/fixtures/closure_capture/move_rebind_cell.ark` が
  wasm32-gc / wasm32 の両経路で `1 2 3 1 4` を出力)。
- non-escaping mutable borrow capture も動作する
  (`borrow_rebind_cell.ark` → `1 3 30`)。
- escaping mutable borrow(closure を fn value として return /
  aggregate 保存 / callback 引数に渡し、body が source を rebind)は
  現行 E0213 で拒否される。
- 対応には親関数の `let mut count` を cell として下ろし、以後の
  read/write を cell 経由にする promotion が必要。closure literal より
  前に binding を下ろすため、function-level の事前解析(または
  typechecker からの binding flag)が必要になる。

## Acceptance

- [x] escaping mutable-borrow capture が source mutation を共有し、
      closure 経由の write が親側の read に見える(逆方向も同様)。
- [x] wasm32-gc と wasm32 linear で同じ意味論・同じ観測結果になる。
- [x] borrow capture を含まない経路の env layout / capture 意味論を
      変えない。
- [x] E0213 の発火条件が「env が保持できない経路」のみに狭まる、
      または採用仕様に従い撤廃される。

## Required evidence

- implementation commit SHA
- ADR-061 の fallback 記述の更新または維持根拠(意味論変更の有無を確定する)
- positive fixture(escaped borrow cell の write-through)+
  引き続き拒否される経路がある場合は negative/diagnostic fixture
- GC/linear の compile/validate/runtime gate 出力

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #872 の担当項目を evidence 付きで更新する

## Open design questions

- promotion 判定の責務: typechecker の binding flag か、mir lowering の
  function-level 事前解析か。closure literal より前に source binding を
  下ろす必要がある点が制約。
- cell 昇格した local の寿命: escape した closure より長く生きる
  heap cell は GC / linear tracing で自然に回収されるか
  (design question — ADR-061 の borrow lifetime 契約との整合)。
- `let` 以外の capture source(関数引数、field 経路)を同じ機構で
  扱うか、引数は対象外にするか。

## Done (2026-09-28)

- implementation: `814759c6a` feat(#891)。non-move rebind capture が
  capture point で source place を共有 heap cell(`__cell_N`)へ昇格し、
  env が cell ref を live alias として運ぶ。payload 型 shadow +
  ctx_emit 段 rewrite により、宣言型・演算 dispatch・返り値解決を
  値型のまま保つ。direct call / env trampoline のどちらも同一 cell ref
  が cap argument として渡り write-through する。
  `src/compiler/mir/lower/closure_pcells.ark` 新設ほか。
- 設計: ADR-081 ACCEPTED、ADR-061 §2/§3 に改訂注記(`756ca4787`)。
  E0213 は shared-borrow capture のみに narrowed され、mutable-borrow
  rebind capture は return / callback 引数 / aggregate 保存の全
  escape 経路で受理される。mutable borrow の排他性・borrow 中の
  source への代入禁止(E0214)・move-after-capture(E0212)は不変。
- positive fixture: `tests/fixtures/closure_capture/escape_borrow_arg.ark`
  → `1 / 1`、`escape_borrow_return.ark` → `1 2 1 3`、
  `escape_borrow_store.ark` → `1 2 2`（いずれも wasm32-gc /
  wasm32 linear で同一出力）。「逆方向」= 親 write → closure read は、
  borrow 生存中の親側代入が E0214 で拒否される設計のため観測不能経路
  (mutable borrow の排他性、ADR-061 から不変)。
- negative/diagnostic: shared-borrow escape は引き続き E0213
  (`closure_borrow_return/store/callback.diag` 維持、実機で発火確認)。
- GC/linear gate: `check-cross-backend-parity.py --filter-dir closure_capture`
  11/11 parity(`.build/parity/closure_capture.json`)。
  `verify fixtures` 全実行 FAIL=260 は pre-891(`331185572`)との
  差分検証で **FAIL 集合が完全一致** — 回帰ゼロ。260 件は進行中の
  v2 epic 既存失敗(Ord/SIMD/stdlib migration 等、別 issue 管轄)。
- 付随修正: `ctx_closure_lookup` に fn-frame scoping を追加。
  producer frame の stale bind が main の同名 let-bound closure へ
  underfunded direct call を emit する既存 invalid-wasm バグを解消
  (move closure でも再現し得た)。
- 残余(別 issue): 中間 closure が捕捉していない変数への多段
  capture が invalid Wasm を生成する既存バグ(pre-891 で同一症状、
  shared/mut borrow 双方)は #900 へ分離。outer の shared-borrow cap
  経由の nested reborrow が snapshot cell として振る舞う意味論は
  ADR-081 再検討条件に記録済み。
