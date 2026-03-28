# Component Model: 複数エクスポート world の自動生成

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 118
**Depends on**: 117
**Track**: wasm-quality
**Blocks v4 exit**: no

## Summary

現在の WIT world 生成はエクスポート関数を平坦なリストとして扱うが、
WASI P2 の `wasi:cli/command`・`wasi:http/proxy` など標準 world への
自動適合 (`use`) をサポートする。
`--world wasi:cli/command` フラグで標準 world にバインドしたコンポーネントを生成する。

## 受け入れ条件

1. `arukellt compile --world wasi:cli/command` で標準 CLI world を生成
2. `--world wasi:http/proxy` で HTTP サーバ world を生成
3. world のインポート不足がある場合に分かりやすいエラーメッセージ
4. wasmtime で各 world のコンポーネントが実行できることを確認

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md` §Component Modelとの関係
