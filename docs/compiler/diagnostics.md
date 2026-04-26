# 診断システム

> **Current-first**: 実装の現在地は [../current-state.md](../current-state.md) を参照してください。
> 仕様を説明するより、現行 registry / fixture / baseline を正とします。

## 現在の方針

- 診断 code は `crates/ark-diagnostics` に正規化されている
- 各 code は **severity** と **phase origin** を持つ
- renderer は phase、primary span、expected/actual、fix hint を出せる
- structured snapshot は tests / baseline / docs 同期用の補助出力

## Phase origin

- `parse`
- `resolve`
- `typecheck`
- `target`
- `backend-validate`
- `internal`

## Canonical warnings / validation codes

| Code | Severity | Phase | Meaning |
|---|---|---|---|
| `W0001` | warning | `typecheck` | same-body heuristic for shared mutable aliasing |
| `W0002` | warning | `target` | deprecated target alias |
| `W0003` | warning | `resolve` | ambiguous import naming |
| `W0004` | error | `backend-validate` | generated Wasm failed validation |

### Important current-first choice

`W0004` はこの refactor で **warning ではなく hard error** として扱う。
backend validation を通らない Wasm を「成功ビルド」として流さないため。

## 現在よく見るカテゴリ

### Parse

- `E0001` unexpected token
- `E0002` missing token
- `E0003` invalid construct

### Resolve

- `E0100` unresolved name
- `E0101` duplicate definition
- `E0102` private access
- `E0103` circular import

### TypeCheck

- `E0200` type mismatch
- `E0202` wrong argument count
- `E0204` non-exhaustive match
- `E0210` incompatible error type for `?`
- `W0001` shared mutable alias heuristic

### Target / Backend

- `E0305` unsupported target
- `E0306` invalid emit kind for target
- `E0307` feature not available for target
- `W0002` deprecated target alias
- `W0004` backend validation failure

## Renderer contract

現在の renderer は少なくとも次を出す。

- `error[E0200|typecheck]: ...` のような header
- primary span
- 型不一致系では `expected` / `actual`
- `fix_it` / suggestion がある場合の help
- notes

## Hidden snapshot / dump support

安定 CLI surface ではなく、開発者向け補助として次を使う。

- `ARUKELLT_DUMP_DIAGNOSTICS=1` — structured diagnostics snapshot
- `ARUKELLT_DUMP_PHASES=parse,resolve,mir` — hidden phase dump

想定 phase 名:

- `parse`
- `resolve`
- `corehir`
- `mir`
- `optimized-mir`
- `backend-plan`

## Internal codes

internal bug reporting 用に `ICE-*` 系 ID を registry に予約する。

- `ICE-PIPELINE`
- `ICE-MIR`
- `ICE-BACKEND`

これは user-facing language error ではなく compiler bug 分類用。

## Snapshot / baseline の使い方

- fixture baseline は `tests/baselines/fixture-baseline.json`
- perf baseline は `tests/baselines/perf-baseline.json`
- API baseline は `tests/baselines/api-baseline.json`
- docs drift は `scripts/check/check-docs-consistency.py`

## 注意

古い文書では:

- `for` 未実装
- trait / method / operator 未実装
- nested generics 未実装
- capability I/O が標準前提
- `W0004` が warning-only

といった前提が混じる。現行 guidance には使わない。

### T3 backend validation details

T3 (`wasm32-wasi-p2`) output passes through the same `wasmparser::Validator::validate_all()`
gate as T1. WasmGC-specific validation covers:

- Heap type indices and subtyping
- GC struct/array field declarations
- Linear memory operations with correct types (i32/i64/f64)
- Function type signatures matching call sites
- `call_indirect` type index correctness

Any `W0004` failure stops the build. The emitter must produce valid WasmGC modules;
there is no fallback or warning-only mode.

### T3 runtime phase errors

T3 runtime errors carry context prefixes for phase-aware triage:

| Phase | Error prefix | Example |
|-------|-------------|---------|
| Engine creation | `engine creation error (GC)` | LLVM config or GC feature unavailable |
| Module compilation | `wasm compile error (GC)` | wasmtime cannot compile the GC module |
| WASI link | `wasi link error` | WASI P1 adapter linking failure |
| Instantiation | `wasm instantiation error (GC)` | Missing imports or memory constraints |
| Execution | `runtime error` | Trap during `_start` execution |
| Entry point | `missing _start` | Module lacks `_start` export |

These prefixes allow distinguishing T3 backend/runtime failures from frontend
(parse/resolve/typecheck) or target-selection (E0305–E0307) errors.

## 関連

- [error-codes.md](error-codes.md) — canonical error code reference (all codes, examples, phases)
- [../current-state.md](../current-state.md)
- [pipeline.md](pipeline.md)
- [../language/error-handling.md](../language/error-handling.md)
- [../process/policy.md](../process/policy.md)
