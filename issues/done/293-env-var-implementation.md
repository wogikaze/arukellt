---
Status: done
Created: 2026-03-31
Updated: 2026-06-12
Closed: 2026-06-12
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

## Reopened by audit — 2026-06-12 (slice D)

**Classification**: `must-reopen` / `implementation-parts-only`

**Reopen reason**: Close note cites deleted Rust `crates/ark-wasm` emit paths and claims full WASI environ integration. The selfhost emitter still stubs `env::var` / `env::get_var` to always return `None`.

**Repo evidence**:

- `src/compiler/wasm/intrinsic_env_args.ark`: `emit_env_var` and `emit_env_get_var` delegate to `emit_env_missing_var_option`.
- `src/compiler/wasm/call_host_io.ark` routes `env::var` / `env_var` to those emitters — no `environ_get` lowering on the selfhost path.
- Resolution references `crates/ark-wasm/src/emit/t3/` which no longer exists (`crates/` absent post #583).

**Violated acceptance**: all three checkboxes (real env lookup, conditional WASI imports, passing lookup fixture on selfhost path).

**Evidence files**: `src/compiler/wasm/intrinsic_env_args.ark`, `std/host/env.ark`, `tests/fixtures/stdlib_env/env_var_lookup.ark`, `scripts/run/arukellt-selfhost.sh`

**Follow-up split**: none

---

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
