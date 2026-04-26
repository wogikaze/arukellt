# --deny-fs の実施と --dir の検証テスト

**Status**: done
**Created**: 2026-03-31
**ID**: 307
**Depends on**: —
**Track**: capability
**Priority**: 14

## Summary

`--dir` フラグは実装済みだが検証テストが薄い。`--deny-fs` は `--dir` を上書きする仕様だが、この相互作用のテストがない。#294 の capability surface 文書化の前に実体を固める。

## Current state

- `crates/arukellt/src/main.rs:123-126`: `--dir` / `--deny-fs` フラグ定義済み
- `crates/arukellt/src/runtime.rs:28-50`: `DirGrant::parse` 実装済み
- `tests/fixtures/stdlib_io/fs_read_write.flags`: `--dir .` 使用
- `--deny-fs` が `--dir` を上書きする動作の検証テストがない
- `--dir` の `:ro` / `:rw` パーミッション指定の検証テストがない

## Acceptance

- [x] `--deny-fs` 指定時に FS アクセスが WASI レベルで拒否されることをテストする fixture
- [x] `--dir path:ro` で書き込みが拒否されることをテストする fixture
- [x] `--deny-fs` が `--dir` を上書きすることをテストする fixture
- [x] 上記テストが harness に登録される

## References

- `crates/arukellt/src/main.rs:123-126`
- `crates/arukellt/src/runtime.rs:28-84`
- `tests/fixtures/stdlib_io/fs_read_write.ark`
