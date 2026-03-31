# Playground: editor / diagnostics UI を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 380
**Depends on**: 379
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 23

## Summary

CodeMirror または Monaco ベースの editor UI を実装し、Wasm 化された parser / formatter / diagnostics と接続する。syntax highlighting、リアルタイム diagnostics 表示、format ボタン、output pane を提供する。

## Current state

- `docs/index.html`: Docsify ベースの static docs shell、editor なし
- VS Code extension (`extensions/arukellt-all-in-one/`) の TextMate grammar が syntax highlighting の source of truth
- playground 用の frontend package なし
- editor / UI component なし

## Acceptance

- [ ] browser editor が Arukellt syntax highlighting 付きで動作する
- [ ] リアルタイムで parse error / check diagnostics が表示される
- [ ] format ボタンで client-side formatter が実行される
- [ ] output pane に diagnostics が structured 表示される
- [ ] mobile responsive である必要はないが、desktop ブラウザで usable であること

## References

- `docs/index.html` — 既存 docs site
- `extensions/arukellt-all-in-one/syntaxes/` — TextMate grammar
- `crates/ark-parser/src/fmt.rs` — formatter
