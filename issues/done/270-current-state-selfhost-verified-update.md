---
Status: done
Created: 2026-03-30
Updated: 2026-03-30
ID: 270
Track: main
Depends on: 266
Orchestration class: implementation-ready
---
# current-state.md の selfhost 記述を verified ベースに更新する
**Blocks v1 exit**: yes

## Summary

`docs/current-state.md` の selfhost 記述が「部品がある」「前進している」という進行形に留まっており、何が verified で何が未達かを一読して判断できない。268/267 で整備した CI 結果に基づいた記述に更新する。

## Acceptance

- [x] `docs/current-state.md` の selfhost セクションが「達成済み項目 / 未達項目」のリスト形式になっている
- [x] 各項目が「CI で継続検証中」か「手動確認のみ」かを明記している
- [x] `docs/compiler/bootstrap.md` の完了条件 checklist への参照リンクが張られている
- [x] 「selfhost 用の部品がある」という記述が除去されている

## Scope

- `docs/current-state.md` の selfhost セクションの書き直し
- 266 の完了条件 checklist に対応した達成状況の記載
- `docs/compiler/bootstrap.md` への参照リンクの追加

## References

- `docs/current-state.md`
- `docs/compiler/bootstrap.md`
- `issues/open/266-selfhost-completion-definition.md`
- `issues/open/253-selfhost-completion-criteria.md`