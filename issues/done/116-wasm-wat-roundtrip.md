# Wasm WAT ラウンドトリップ検証 (wat2wasm ⇄ wasm2wat)

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-14
**ID**: 116
**Depends on**: 114
**Track**: wasm-quality
**Blocks v4 exit**: no

---

## Summary

生成した Wasm バイナリを `wasm2wat` でテキスト形式に変換し、
さらに `wat2wasm` でバイナリに変換して WAT テキストが一致することを確認する。
これにより T1 emitter が生成するバイナリの well-formedness を保証する。

## 受け入れ条件

- [x] `scripts/run/verify-harness.sh` に WAT ラウンドトリップチェックを追加 (`--wat` flag → `scripts/run/wat-roundtrip.sh`)
- [x] 全 fixture について roundtrip チェックが差分ゼロ (311 `run:` エントリ中 300 PASS, 11 SKIP — compile 不可 fixture)
- [x] ラウンドトリップ失敗時のエラー診断を `scripts/run/wat-roundtrip.sh` のコメントに文書化

## Implementation notes

- `wasm-tools` が不在の場合は wabt (`wasm2wat` / `wat2wasm`) にフォールバック。両方不在の場合は graceful skip (exit 0)。
- wabt は Component Model (T3 / wasm32-wasi-p2) の型形式 (0x5e 等) を解釈できないため、wabt 使用時は T1 (wasm32-wasi-p1) のみテスト。wasm-tools 存在時は T3 も対象。
- 失敗パターン: `wasm2wat failed` (ill-formed binary), `wat2wasm failed` (invalid WAT), `WAT text differs` (lossy encoding)。

## 参照

- `docs/spec/spec-1.0.0/OVERVIEW.md` §コア仕様
- `scripts/run/wat-roundtrip.sh`
