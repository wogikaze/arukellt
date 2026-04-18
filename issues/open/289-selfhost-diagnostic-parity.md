# セルフホスト diagnostic parity を確認する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-15
**ID**: 289
**Depends on**: 287
**Track**: selfhost
**Orchestration class**: verification-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no
**Priority**: 9


## Completion — 2026-04-15

Parity verification re-run against current `.build/selfhost/arukellt-s1.wasm` (377 kB)
and `target/debug/arukellt`. Results: 30 fixtures — pass=0, fail=0, skip=30.
All gaps documented in `docs/compiler/diagnostic-parity.md` with fresh data.
`verify-harness.sh --quick` passes (19/19). Pre-existing `cargo test` failures in
stdlib_cli/stdlib_csv/stdlib_hashmap are unrelated to diagnostic parity.

## Reopened by audit — 2026-04-13

**Reason**: Not strict parity.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Summary

エラーメッセージ (code, position, text) が Rust 版と selfhost 版で同等か未検証。diagnostic parity は dual-period 終了条件の一つ。

## Current state

- `docs/compiler/bootstrap.md:92`: diagnostic parity が終了条件に列挙
- selfhost の error reporting は `driver.ark` の CompileResult に集約
- Rust 版は `crates/ark-diagnostics/` を使用

## Acceptance

- [x] 代表的なエラーケース（未定義変数、型不一致、構文エラー）で両版の出力を比較
- [x] 比較契約: error code / primary span (行番号) / severity の一致を求める。message 文言の完全一致は求めない
- [x] 差分リストが作成される

## References

- `docs/compiler/bootstrap.md`
- `src/compiler/driver.ark`
- `crates/ark-diagnostics/`
