---
Status: open
Created: 2026-06-29
Updated: 2026-10-23
ID: 708
Track: stdlib-api
Depends on: ""
Orchestration class: normal
Orchestration upstream: ""
Blocks v{N}: none
Priority: 2
Source: Issue #692 deferral — TryFrom/TryInto deferred pending Self + Result return type resolution
---

# 708 — `TryFrom` / `TryInto` traits for fallible conversions

## Summary

Fallible conversion の trait を追加する。ADR-084 により、正規形は
`TryFrom` の **associated function** (`U::try_from(value)`) とし、
`TryInto` の method 形式 (`x.try_into()`) は target 型推論の採択まで保留する。

`Result<T, E>` は prelude に存在する。残る問題は trait dispatch の戻り型解決ではなく、
associated function の call-result temp に let 注釈の `Result` 型を伝播させる
codegen 側の型伝播である。

## Current state

- `Result<T, E>` / `Option<T>` は prelude に存在する。
- `From<T>` / `Into<T>` は #692 で実装済み。
- #707 (Self return type) と #701 (associated function syntax) は close 済み。
- `i32::try_from("x")` の associated 形式は typecheck を通る（probe）。
- 同じ probe の call-result temp `_t2` が i64 宣言になり、callee が返す
  `Result<i32, String>` ref と一致しない。let 注釈の型/mvt を initializer temp へ
  伝播する compiler 修正が必要。
- `String` に対する複数の `TryInto<i32/i64/f64>` impl を用いた
  `"x".try_into()` は、期待型から impl を選べず先頭 impl を選ぶため typecheck に失敗する。
  このため method 形式は採択しない（ADR-084）。

## Required work

### Compiler

- [ ] associated trait function の具象戻り型 `Result<U, String>` を call site / signature registry で解決する。
- [ ] let 注釈が付いた call-result temp に、注釈の `Result`/`Option` 型と mvt を伝播する
      （`src/compiler/mir/lower/stmt_let_store.ark`）。
- [ ] probe と下記 fixture を wasm32-gc で compile / validate / run する。

### Stdlib

- [ ] `std/core/convert.ark` に
      `trait TryFrom<T> { fn try_from(value: T) -> Result<TryFrom, String> }` を定義する。
- [ ] 次の impl を追加する。
  - `impl TryFrom<String> for i32` — `parse_i32` 相当
  - `impl TryFrom<String> for i64` — `parse_i64` 相当
  - `impl TryFrom<String> for f64` — `parse_f64` 相当
  - `impl TryFrom<i64> for i32` — 範囲検査付き narrowing
- [ ] `TryInto` method は追加しない（ADR-084 で保留）。

### Fixtures

- [ ] `tests/fixtures/stdlib_trait/try_from_parse.ark` —
      `let n: Result<i32, String> = i32::try_from("42")` が `Ok(42)` になり、
      `i64` / `f64` の parse も成功する。
- [ ] `tests/fixtures/stdlib_trait/try_from_narrow.ark` —
      `i32::try_from(999999999999i64)` が `Err`、範囲内の `i32::try_from(42i64)` が `Ok(42)` になる。
- [ ] manifest に `run:` / `t3-run:` を登録する。
- [ ] `python3 scripts/manager.py verify quick` exits 0。

## Acceptance

- [ ] `i32::try_from("42")` returns `Ok(42)` as `Result<i32, String>`.
- [ ] `i32::try_from(999999999999i64)` returns `Err` as `Result<i32, String>`.
- [ ] `TryFrom` trait and scalar impls are defined; `TryInto` method is not required here.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## References

- ADR: [ADR-084](../../docs/adr/ADR-084-language-v2-fallible-conversion-dispatch.md)
- Depends on (closed): #692 (From/Into — base conversion traits),
  #707 (Self return type), #701 (associated function syntax)
- Related: #690 (`?` operator — stays `From`-based per ADR-063),
  #694 (Error trait — associated error type remains future scope)
- `std/core/convert.ark` — existing From/Into definitions and parse helpers
- `std/prelude.ark` — `Result<T, E>`, `parse_i32`, `parse_i64`, `parse_f64`
- `src/compiler/mir/lower/stmt_let_store.ark` — let annotation type propagation
