# 167: Arukellt 版コンパイラの ARUKELLT_DUMP_PHASES 実装

**Version**: v5
**Priority**: P2
**Depends on**: #163 (Driver + CLI)

## 概要

Arukellt 版コンパイラに `ARUKELLT_DUMP_PHASES` 相当のデバッグ出力を実装する。セルフホスト中のデバッグに必須。

## タスク

1. CLI 引数 `--dump-phases=tokens,ast,hir,mir,wasm` の追加
2. 各フェーズの中間出力を stderr に書き出す:
   - `tokens`: トークン列のダンプ
   - `ast`: AST のテキスト表現
   - `hir`: 型付き HIR のダンプ
   - `mir`: MIR のダンプ
   - `wasm`: 生成 Wasm の WAT テキスト (wasmprinter 相当)
3. Rust 版と同一フォーマットで出力 (compare-outputs.sh で比較するため)

## 完了条件

- `--dump-phases=tokens` で Rust 版と同一のトークンダンプが出力される
- `--dump-phases=ast` で Rust 版と同一の AST ダンプが出力される
- Phase 1 完了時点で tokens, ast が利用可能

## 注意事項

- デバッグ出力は stderr に書く (stdout は Wasm バイナリ出力に使うため)
- フォーマットの一致は fixpoint デバッグの生命線
