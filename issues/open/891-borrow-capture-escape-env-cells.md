---
Status: open
Created: 2026-09-27
Updated: 2026-09-27
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

- [ ] escaping mutable-borrow capture が source mutation を共有し、
      closure 経由の write が親側の read に見える(逆方向も同様)。
- [ ] wasm32-gc と wasm32 linear で同じ意味論・同じ観測結果になる。
- [ ] borrow capture を含まない経路の env layout / capture 意味論を
      変えない。
- [ ] E0213 の発火条件が「env が保持できない経路」のみに狭まる、
      または採用仕様に従い撤廃される。

## Required evidence

- implementation commit SHA
- ADR-061 の fallback 記述の更新または維持根拠(意味論変更の有無を確定する)
- positive fixture(escaped borrow cell の write-through)+
  引き続き拒否される経路がある場合は negative/diagnostic fixture
- GC/linear の compile/validate/runtime gate 出力

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #872 の担当項目を evidence 付きで更新する

## Open design questions

- promotion 判定の責務: typechecker の binding flag か、mir lowering の
  function-level 事前解析か。closure literal より前に source binding を
  下ろす必要がある点が制約。
- cell 昇格した local の寿命: escape した closure より長く生きる
  heap cell は GC / linear tracing で自然に回収されるか
  (design question — ADR-061 の borrow lifetime 契約との整合)。
- `let` 以外の capture source(関数引数、field 経路)を同じ機構で
  扱うか、引数は対象外にするか。
