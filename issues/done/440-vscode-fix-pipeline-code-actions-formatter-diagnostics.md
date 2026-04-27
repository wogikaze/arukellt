---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 440
Track: vscode-ide
Depends on: 341, 346, 348, 349, 350, 352
Orchestration class: implementation-ready
Blocks v1 exit: False
Priority: 2
# VSCode Extension: Code Actions・Formatter・Diagnostics を統合し「fix-allが意味を持つ」状態にする
---
# VSCode Extension: Code Actions・Formatter・Diagnostics を統合し「fix-allが意味を持つ」状態にする

## Summary

現在バラバラに存在する formatter / code actions / diagnostics を統合し、VSCode上での「自動修正」が一貫した意味を持つようにする。import整理・未使用削除・lint修正などを単一の fix pipeline に統合する。

## Current state

- formatter が import 整理も担っている。
- code actions は限定的（auto-import 程度）。
- diagnostics と fix の対応が弱い。
- fix-all が実質 formatter 呼び出し。

## Acceptance

- [x] formatter と semantic fix を分離する。
- [x] unused import / dead binding の自動修正が可能。
- [x] `source.fixAll` が複数ルールを統合して適用される。
- [x] CLI と LSP で同じ修正結果になる。
- [x] 修正差分が snapshot テストで固定される。

## References

- `crates/ark-parser/src/fmt.rs`
- `crates/ark-lsp/src/server.rs`
- `crates/ark-diagnostics/`
- `crates/arukellt/src/commands.rs`