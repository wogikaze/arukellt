# env::var() の実装を完成させる

**Status**: open
**Created**: 2026-03-31
**ID**: 293
**Depends on**: —
**Track**: main
**Priority**: 13

## Summary

`std/host/env.ark` の `var()` 関数が常に `None` を返す stub のまま。WASI の `environ_sizes_get` / `environ_get` を使って実際の環境変数を取得すべき。

## Current state

- `std/host/env.ark:23-25`: `var()` が `None` を返す stub
- `args()` / `arg_count()` は intrinsic 経由で実装済み
- T3 emitter の reachability scan に env 関連の検出がある

## Acceptance

- [ ] `env::var("HOME")` 等が実際の環境変数値を返す
- [ ] WASI `environ_sizes_get` / `environ_get` import が条件付きで追加される
- [ ] テスト: `env::var` が値を返す fixture（`--env` フラグまたは WASI 環境変数経由）

## References

- `std/host/env.ark`
- `crates/ark-wasm/src/emit/t3/reachability.rs`
