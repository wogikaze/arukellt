---
Status: done
Created: 2026-03-31
Updated: 2026-06-28
Track: capability
Orchestration class: implementation-ready
Depends on: —
Closed: 2026-06-28
ID: 293
Blocks v1 exit: no
Priority: 13
---

# env: ":var() の実装を完成させる"
- T3 (WasmGC): Full `environ_sizes_get`/`environ_get` integration with byte-level key comparison and `Option_String` wrapping. Environ buffer placed past data segments to avoid memory corruption.
- T1: "Stub returning `None` (WASI environ not yet wired for linear memory target)."
- Fixed multiple T3 bugs: "Option_String variant name remapping, MIR enum_typed_locals specialization, runtime `inherit_env()`."
- Test fixture: "`env_var_lookup.ark` (T3-only) verifies PATH lookup and nonexistent var handling."
- [x] `env: ":var("HOME")` 等が実際の環境変数値を返す"
- [x] テスト: "`env::var` が値を返す fixture（`--env` フラグまたは WASI 環境変数経由）"
# env::var() の実装を完成させる

## Summary

`std/host/env.ark` の `var()` 関数が常に `None` を返す stub のまま。WASI の `environ_sizes_get` / `environ_get` を使って実際の環境変数を取得すべき。

## Resolution

Implemented full WASI environ integration:

- T3 (WasmGC): Full `environ_sizes_get`/`environ_get` integration with byte-level key comparison and `Option_String` wrapping. Environ buffer placed past data segments to avoid memory corruption.
- T1: Stub returning `None` (WASI environ not yet wired for linear memory target).
- Added `__intrinsic_env_var` to resolver prelude, typechecker builtins, and stdlib manifest.
- Fixed multiple T3 bugs: Option_String variant name remapping, MIR enum_typed_locals specialization, runtime `inherit_env()`.
- Test fixture: `env_var_lookup.ark` (T3-only) verifies PATH lookup and nonexistent var handling.

## Acceptance

- [x] `env::var("HOME")` 等が実際の環境変数値を返す
- [x] WASI `environ_sizes_get` / `environ_get` import が条件付きで追加される
- [x] テスト: `env::var` が値を返す fixture（`--env` フラグまたは WASI 環境変数経由）

## References

- `std/host/env.ark`
- `crates/ark-wasm/src/emit/t3/reachability.rs`
- `crates/ark-wasm/src/emit/t3/stdlib.rs`
- `tests/fixtures/stdlib_env/env_var_lookup.ark`