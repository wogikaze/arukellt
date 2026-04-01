# Selfhost compiler の出力契約を統一する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2025-07-15
**ID**: 321
**Depends on**: 319
**Track**: selfhost-cli
**Blocks v1 exit**: no
**Priority**: 13

## Summary

selfhost compiler のエラー出力形式、終了コード、構造化出力を Rust 版と揃える。現在 selfhost のエラーは flat text で位置情報のフォーマットが Rust 版と異なる。parity 検証の前提として出力契約を定義する。

## Current state

- selfhost のエラー出力: flat text (file name + message のみ)
- Rust 版のエラー出力: `file:line:col: error[E0001]: message` 形式
- selfhost の終了コード: 未定義 (常に 0 の可能性あり)
- Rust 版: 0 (成功) / 1 (compile error) / 2 (usage error)
- `--json` フラグ: Rust 版のみ、selfhost にはない

## Acceptance

- [x] compile error の出力が `file: error: message` 形式になる
- [x] 終了コード: 0 (成功) / 1 (compile error) / 2 (usage error) が統一される
- [x] `--json` フラグで JSON 形式の diagnostic を出力する
- [x] 正常終了時の stdout / stderr が Rust 版と同じ契約に従う (stderr for progress, stdout for data)

## Verification

- `arukellt check src/compiler/main.ark` → OK
- `verify-bootstrap.sh --stage1-only` → PASS (9/9 compiled, 84798 bytes)
- Exit codes: usage error → 2, compile error → 1, success → 0
- JSON output: `--json` produces `{"file":"...","phase":...,"diagnostics":[...]}` or `{"file":"...","success":true,"output_size":...}`

## References

- `src/compiler/driver.ark` — selfhost error reporting
- `crates/ark-diagnostics/` — Rust diagnostic formatting
- `crates/arukellt/src/main.rs` — Rust 終了コード
