# Wasm 検証エラーをエラーに昇格 (W0004 warning → error)

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 114
**Depends on**: —
**Track**: wasm-quality
**Blocks v4 exit**: yes

## Summary

現在 `crates/ark-wasm/src/emit/mod.rs` の Wasm バイナリ検証 (wasmparser) の失敗は
`W0004` warning として扱われている (TODO コメント付き)。
v4 では全ての生成 Wasm が wasmparser の検証を通ることを保証し、
検証失敗をコンパイルエラーに昇格する。

## 受け入れ条件

1. `ark-wasm/src/emit/mod.rs` の wasmparser 検証失敗を `Err` として返す
2. 既存のすべての fixture が検証を通過する
3. `scripts/verify-harness.sh` に wasmparser 検証ゲートを追加
4. 検証エラーが発生した場合の診断メッセージ改善

## 参照

- `crates/ark-wasm/src/emit/mod.rs:34` (W0004 TODO)
status: done
closed: 2026-03-28
resolution: W0004 already promoted to Severity::Error in ark-diagnostics. Wasm validation is a hard error at backend-validate phase.
