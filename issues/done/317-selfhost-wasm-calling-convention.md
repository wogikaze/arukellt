---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 317
Track: selfhost-backend
Depends on: 316
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 10
---

# Selfhost Wasm emitter: 呼び出し規約と WASI import を実装する
- `src/compiler/emitter.ark`: fd_write import の type index を placeholder で書いている
# Selfhost Wasm emitter: 呼び出し規約と WASI import を実装する

## Summary

関数呼び出しの calling convention、WASI fd_write / proc_exit 等の import section 生成、memory layout (string pointer + length) を実装する。これにより selfhost compiler が「hello world を wasmtime で実行可能な .wasm に compile する」最小ゴールを達成する。

## Current state

- `src/compiler/emitter.ark`: fd_write import の type index を placeholder で書いている
- 値型変換で `ref -> i32 placeholder` が残っている
- export は `_start` と `memory` のみ (最小形)
- WASI import の function index 割り当てが hardcoded
- 関数間呼び出しの type section 生成が不完全

## Acceptance

- [x] function call が正しい type index で `call` 命令を出す
- [x] fd_write import が import section に正しく登録される
- [x] string 引数が linear memory 上のポインタ+長さペアで渡される
- [x] proc_exit(0) による正常終了が動作する
- [x] selfhost compiler で hello.ark を compile し wasmtime で "Hello" が出力される

## References

- `src/compiler/emitter.ark` — selfhost emitter (fd_write placeholder)
- `crates/ark-wasm/src/emit/t1/mod.rs` — Rust T1 import/export 生成
- `crates/ark-wasm/src/emit/t1/helpers.rs` — memory layout, calling convention