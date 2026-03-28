# Wasm Name Section: デバッグ用関数名・ローカル名セクション生成

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 115
**Depends on**: —
**Track**: wasm-quality
**Blocks v4 exit**: no

## Summary

生成する Wasm バイナリに Name Section (custom section `name`) を追加し、
wasmtime のスタックトレースや `wasm-objdump` でのデバッグ体験を改善する。
`--opt-level 0` では名前情報を完全に含め、`--opt-level 2` では省略可能とする。

## 受け入れ条件

1. T3 emitter が Name Section に Ark 関数名をエクスポート
2. ローカル変数にも名前を付与 (`--opt-level 0` でのみ)
3. wasmtime のスタックトレースで Ark 関数名が表示されることを確認
4. `--strip-debug` フラグで Name Section を省略

## 参照

- WebAssembly binary format §custom section
