# Playground: editor / diagnostics UI を実装する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-03
**ID**: 380
**Depends on**: 379
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 23

## Status note (2026-04-03)

この issue は historical implementation-parts work としては done のまま保持するが、**current repo で browser-reachable playground が存在する証拠としては使わない**。

- done の範囲は `playground/src/**` 周辺の editor / diagnostics component work に限定する。
- repo-visible browser entrypoint / route、publish path、docs alignment、extension exposure はこの issue の完了証拠ではない。
- current product-proof tracking は `issues/open/465-playground-false-done-audit-and-status-rollback.md` と `issues/open/466`〜`472` で行う。

## Summary

CodeMirror または Monaco ベースの editor UI を実装し、Wasm 化された parser / formatter / diagnostics と接続する。syntax highlighting、リアルタイム diagnostics 表示、format ボタン、output pane を提供する。

## Current state

- `docs/index.html`: Docsify ベースの static docs shell、editor なし
- VS Code extension (`extensions/arukellt-all-in-one/`) の TextMate grammar が syntax highlighting の source of truth
- playground 用の frontend package なし
- editor / UI component なし

## Acceptance

- [x] browser editor が Arukellt syntax highlighting 付きで動作する
- [x] リアルタイムで parse error / check diagnostics が表示される
- [x] format ボタンで client-side formatter が実行される
- [x] output pane に diagnostics が structured 表示される
- [x] mobile responsive である必要はないが、desktop ブラウザで usable であること

## References

- `docs/index.html` — 既存 docs site
- `extensions/arukellt-all-in-one/syntaxes/` — TextMate grammar
- `crates/ark-parser/src/fmt.rs` — formatter
