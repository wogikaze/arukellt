---
Status: done
Created: 2026-04-03
Updated: 2026-04-10
ID: 484
Track: compiler
Depends on: 483
Orchestration class: implementation-ready
---
# compiler: to_string() builtin 実装と stdlib surface 整備
**Blocks v1 exit**: no

---

## Decomposed from 171

Issue 171 (`canonical-to-string-surface`) の **implementation layer** を担当する。
ADR (#483) で `to_string(x)` が canonical と決まった後に、
compiler / emitter / stdlib で実際に動作する実装を行う。

Upstream: #483 (ADR Decided) — 完了後に着手
Downstream: #171 (docs + fixture coverage) — この issue 完了後に着手

---

## Summary

ADR-0xx (#483) の決定に従い、`to_string(x)` を builtin function として実装する。

対象:
- `to_string(i32)` / `to_string(i64)` / `to_string(f64)` / `to_string(bool)` / `to_string(char)` — primitive scalars
- `to_string(String)` — no-op passthrough
- Display-based struct に対する `to_string(x)` の dispatch (あれば)
- `std/core/` または `std/text/` の public surface として export

また `crates/ark-stdlib/src/lib.rs` の `ManifestFunction` に `to_string` を登録し、
LSP / docs 生成がこの関数を公式 surface として認識できるようにする。

## Why this is a separate issue

実装 diff (Rust/compiler + stdlib ark ファイル + manifest) と
docs 更新 (#171) および ADR (#483) を分離することで:
- 実装なしに docs や manifest だけを更新して「done」にする事態を防ぐ
- 実装 PR に ADR 変更や docs 変更が混入しない

## Visibility

user-visible (ユーザーが `to_string(x)` を書いて動作することが要件)

## Primary paths

- `std/core/` または `std/text/` — to_string surface の ark ファイル
- `crates/ark-stdlib/src/lib.rs` — ManifestFunction 登録
- `crates/ark-resolve/src/load.rs` — builtin 解決
- `crates/ark-wasm/src/emit/` — to_string の emit

## Allowed adjacent paths

- `tests/fixtures/` — to_string 動作確認 fixture

## Non-goals

- ADR の作成 (#483)
- docs/quickstart/cookbook の更新 (#171)
- `i32_to_string` 等の既存ヘルパーの廃止 (別途 deprecation issue が必要)
- method syntax `.to_string()` の実装
- Display trait / Display-based dispatch の完全実装

## Acceptance

1. `to_string(42)` が `"42"` を返す Arukellt コードがコンパイル・実行できる
   (fixture `tests/fixtures/stdlib_core/to_string_i32.ark` で確認)
2. `to_string(3.14)` が `"3.14"` を返す (fixture で確認)
3. `to_string(true)` が `"true"` を返す (fixture で確認)
4. `crates/ark-stdlib/src/lib.rs` の manifest に `to_string` 関数が登録されている
5. `cargo test -p arukellt --test harness` の to_string fixture が pass する

## Required verification

- `grep "to_string" crates/ark-stdlib/src/lib.rs` が manifest 登録行を返す
- `tests/fixtures/stdlib_core/to_string_i32.ark` が存在し fixture harness で pass
- `bash scripts/run/verify-harness.sh --quick` が pass

## Close gate

- `to_string` の 3 種類の primitive fixture (i32, f64, bool) が harness で pass
- manifest 登録が `crates/ark-stdlib/src/lib.rs` に存在する
- docs 更新は #171 の担当; docs なしでもこの issue は close できる

## Evidence to cite when closing

- `tests/fixtures/stdlib_core/to_string_*.ark` の fixture ファイル一覧
- `crates/ark-stdlib/src/lib.rs` の to_string 登録行番号
- `cargo test -p arukellt --test harness -- to_string` の pass 出力

## Close evidence (実装完了)

- `tests/fixtures/stdlib_core/to_string_i32.ark` — `to_string(42)` → `"42"` (passthrough String confirmed)
- `tests/fixtures/stdlib_core/to_string_f64.ark` — `to_string(3.14)` → `"3.14"`
- `tests/fixtures/stdlib_core/to_string_bool.ark` — `to_string(true)` → `"true"`
- `crates/ark-stdlib/src/lib.rs` line 61: `name: "to_string".into()` (ManifestFunction registered)
- `std/prelude.ark`: `to_string` polymorphic builtin documented in comment block
- `cargo test -p arukellt --test harness`: 1 passed, 0 failed
- `bash scripts/run/verify-harness.sh --quick`: 19/19 passed

## False-done risk if merged incorrectly

- `i32_to_string` という別名が動いているだけで「to_string が動く」と close する
  → acceptance 1 で `to_string(42)` という呼び出し形式を明示
- manifest 登録だけで「to_string が使える」と docs に書く
  → docs は #171 の担当; この issue は実装のみ