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
- `i32::try_from("42")` / `i64::try_from("9001")` / `f64::try_from("1.5")` の
  associated 形式は typecheck / codegen / run を通る（try_from_parse.ark）。
- let 注釈 `Result<U, String>` の型/mvt は call-result temp へ伝播し、
  emitter は Result/Option/enum local を scalar に縮退させない。
- `i32::try_from(String)` と `i32::try_from(i64)` は同じ mangled `i32::try_from`
  を持ち、callee lookup が先頭 impl を選ぶ。そのため `i64 -> i32` narrowing は
  まだ codegen できない。source 型を mangled / dispatch に含める必要がある。
- `"x".try_into()` の method 形式は期待型から impl を選べないため採択しない
  （ADR-084）。

## Required work

### Compiler

- [x] associated trait function の具象戻り型 `Result<U, String>` を call site / signature registry で解決する。
- [x] let 注釈が付いた call-result temp に、注釈の `Result`/`Option` 型と mvt を伝播する
      （`src/compiler/mir/lower/stmt_let_store.ark`）。
- [x] probe と `try_from_parse.ark` を wasm32-gc で compile / validate / run する。

### Stdlib

- [x] `std/core/convert.ark` に
      `trait TryFrom<T> { fn try_from(value: T) -> Result<TryFrom, String> }` を定義する。
- [x] 次の impl を追加する。
  - `impl TryFrom<String> for i32` — `parse_i32` 相当
  - `impl TryFrom<String> for i64` — `parse_i64` 相当
  - `impl TryFrom<String> for f64` — `parse_f64` 相当
  - `impl TryFrom<i64> for i32` — 範囲検査付き narrowing（定義済み。same-target
    mangling のため call は未対応）
- [x] `TryInto` method は追加しない（ADR-084 で保留）。

### Fixtures

- [x] `tests/fixtures/stdlib_trait/try_from_parse.ark` —
      `let n: Result<i32, String> = i32::try_from("42")` が `Ok(42)` になり、
      `i64` / `f64` の parse も成功する。
- [ ] `tests/fixtures/stdlib_trait/try_from_narrow.ark` —
      `i32::try_from(999999999999i64)` が `Err`、範囲内の `i32::try_from(42i64)` が `Ok(42)` になる。
      same-target mangling の修正待ち。
- [x] manifest に `run:` / `t3-run:` を登録する（try_from_parse）。
- [ ] `python3 scripts/manager.py verify quick` exits 0。

## Acceptance

- [x] `i32::try_from("42")` returns `Ok(42)` as `Result<i32, String>`.
- [ ] `i32::try_from(999999999999i64)` returns `Err` as `Result<i32, String>`.
- [x] `TryFrom` trait and String impls are defined; `TryInto` method is not required here.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Remaining

- `i32::try_from(String)` と `i32::try_from(i64)` を source 型込みの mangled name /
  dispatch で区別し、narrowing fixture を通す。
- `TryInto` method は ADR-084 の target-type-directed resolution 待ち。

## References

- ADR: [ADR-084](../../docs/adr/ADR-084-language-v2-fallible-conversion-dispatch.md)
- Depends on (closed): #692 (From/Into — base conversion traits),
  #707 (Self return type), #701 (associated function syntax)
- Related: #690 (`?` operator — stays `From`-based per ADR-063),
  #694 (Error trait — associated error type remains future scope)
- `std/core/convert.ark` — existing From/Into definitions and parse helpers
- `std/prelude.ark` — `Result<T, E>`, `parse_i32`, `parse_i64`, `parse_f64`
- `src/compiler/mir/lower/stmt_let_store.ark` — let annotation type propagation
