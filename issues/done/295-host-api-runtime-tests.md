# host API の run-time テストを拡充する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 295
**Depends on**: —
**Track**: capability
**Blocks v1 exit**: no
**Priority**: 15

## Summary

使用可能な host API に対する CI テストが不十分。clock, random, fs, env の各モジュールで、正常系・異常系の fixture を追加する。

## Current state

- `tests/fixtures/stdlib_io/clock_random.ark`: clock + random の最小テスト
- `tests/fixtures/stdlib_io/fs_read_write.ark`: fs の読み書きテスト
- `tests/fixtures/stdlib_env/env_basic.ark`: env の最小テスト
- process::exit の正常系テストがない

## Acceptance

- [ ] clock: 2回呼び出しで単調増加を確認する fixture
- [ ] random: API 呼び出しが成功し、返り値が i32 範囲内であることを確認する fixture（非決定性に依存しない）
- [ ] fs: 存在しないファイルの読み取りエラーを確認する fixture（既存 `fs_read_error.ark` で可）
- [ ] env: arg_count / args の引数受け渡しを確認する fixture
- [ ] process: exit(0) の正常終了を確認する fixture
- [ ] 全テストが CI harness に登録される

## References

- `tests/fixtures/stdlib_io/`
- `tests/fixtures/stdlib_env/`
- `tests/fixtures/manifest.txt`
