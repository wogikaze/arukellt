# --deny-clock / --deny-random を compile-time 検証に引き上げる

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 291
**Depends on**: —
**Track**: capability
**Blocks v1 exit**: no
**Priority**: 11

## Summary

`--deny-clock` と `--deny-random` は CLI で受け付けるが、現在は hard error で即終了するだけ。compile-time に対象 import の使用を検出して拒否する仕組みが必要。

## Current state

- `crates/arukellt/src/commands.rs:507-521`: フラグ使用時に `eprintln` + `process::exit(1)`
- `crates/arukellt/src/runtime.rs:11-13`: `RuntimeCaps` に `deny_clock` / `deny_random` フィールドあり
- `crates/arukellt/src/runtime.rs:69-72`: `let _ = caps.deny_clock` で未使用

## Acceptance

- [ ] `--deny-clock` 指定時、clock 関連 intrinsic を使うコードが compile error になる
- [ ] `--deny-random` 指定時、random 関連 intrinsic を使うコードが compile error になる
- [ ] reachability scan (`crates/ark-wasm/src/emit/t3/reachability.rs`) の結果を deny チェックに利用
- [ ] テスト: clock 使用コードが `--deny-clock` で compile fail する fixture

## References

- `crates/arukellt/src/commands.rs:507-521`
- `crates/arukellt/src/runtime.rs`
- `crates/ark-wasm/src/emit/t3/reachability.rs:115-127, 191-203`
