# Wasm WAT ラウンドトリップ検証 (wat2wasm ⇄ wasm2wat)

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 116
**Depends on**: 114
**Track**: wasm-quality
**Blocks v4 exit**: no

## Summary

生成した Wasm バイナリを `wasm2wat` (wasm-tools) でテキスト形式に変換し、
さらに `wat2wasm` でバイナリに変換して元と同一になることを確認する。
これにより T3 emitter が生成するバイナリの well-formedness を保証する。

## 受け入れ条件

1. `scripts/verify-harness.sh` に WAT ラウンドトリップチェックを追加
2. 全 fixture について `wasm2wat | wat2wasm | binary_diff` が差分ゼロ
3. ラウンドトリップ失敗 fixture のエラー診断ヘルパーを追加

## 参照

- `docs/spec/spec-1.0.0/OVERVIEW.md` §コア仕様
