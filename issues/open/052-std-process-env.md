# std::process + std::env + std::cli: 実行環境 API

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 052
**Depends on**: 039, 042
**Track**: stdlib
**Blocks v3 exit**: yes

## Summary

CLI ツール開発に必要な実行環境 API を実装する。
コマンドライン引数取得、環境変数アクセス、プロセス終了、
exit code 制御を std::process, std::env, std::cli として提供する。

## 受け入れ条件

### std::process

```ark
pub fn exit(code: i32) -> !
pub fn abort() -> !
```

### std::env

```ark
pub fn args() -> Vec<String>
pub fn var(name: String) -> Option<String>
pub fn vars() -> Vec<(String, String)>
pub fn current_dir() -> Result<Path, Error>
```

### std::cli (将来の引数パーサ基盤)

```ark
pub fn arg_at(index: i32) -> Option<String>
pub fn arg_count() -> i32
pub fn has_flag(flag: String) -> bool  // --flag の有無
```

## 実装タスク

1. `std/process/process.ark`: exit/abort (WASI proc_exit)
2. `std/env/env.ark`: args/var/vars (WASI P2 cli/environment)
3. `std/cli/cli.ark`: 引数ヘルパー (source 実装、args() を内部使用)
4. `ark-wasm/src/emit`: WASI P2 `wasi:cli/environment` import
5. 旧 `args()` 関数を deprecated 化
6. T1 での fallback: args は空 Vec を返す、var は None を返す

## 検証方法

- fixture: `stdlib_process/exit_zero.ark`, `stdlib_env/args_basic.ark`,
  `stdlib_env/env_var.ark`, `stdlib_cli/flag_check.ark`,
  `stdlib_process/exit_nonzero.ark`

## 完了条件

- `args()` が WASI 環境でコマンドライン引数を返す
- `exit(0)` で正常終了する
- fixture 5 件以上 pass

## 注意点

1. WASI sandbox 内では環境変数アクセスが制限される — preopens に依存
2. `exit()` の戻り値型 `!` (Never) が型システムでどう扱われるか確認
3. T1/T3 間で args 取得の WASI ABI が異なる

## ドキュメント

- `docs/stdlib/process-env-reference.md`

## 未解決論点

1. subprocess 実行 (`command()`/`spawn()`) を v3 に入れるか (v4 送り推奨)
2. signal handling の扱い
