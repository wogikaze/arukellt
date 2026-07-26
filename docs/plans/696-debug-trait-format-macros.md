# #696 — `Debug` trait and `format!` / `write!` equivalent クローズ計画

ステータス: 完了  
親 issue: [#696](../../issues/open/696-debug-trait-format-macros.md)  
前提: #688, #692 done  
担当 subagent lane: `wave/696-debug-trait`  
作業 worktree: `.worktrees/wave-696-debug-trait`  
作成日: 2026-07-25

## 1. 現状とゴール

- `Display` / `Debug` traits と scalar impls は `std/core/convert.ark` に存在。
- f-string は `f"{x}"` までで、debug specifier `f"{x:?}"` が未実装。
- `std/test/mod.ark` は型付き `assert_eq_i32` 等。ジェネリック `assert_eq<T: Eq + Debug>` がない。
- 目標:
  - `f"{x:?}"` 構文をパースし `fmt::format_debug(x)` に脱糖。
  - ジェネリック `assert_eq<T: Eq + Debug>` を追加し、失敗時に Debug 表現を表示。
  - `std/prelude.ark` のレガシー `assert_eq` を削除または置換（ADR-046）。

## 2. 前提・依存

- #688, #692 done。
- `Eq` trait はスカラー型にのみ実装済み。コンテナ/構造体は未実装。

## 3. フェーズと完了条件

### Phase 1 — f-string debug specifier
- `src/compiler/lexer/ident.ark` で `:` 文字をパース。
- `src/compiler/parser/fstring_segments.ark` で `:?` を検出し expr ノードにフラグ付与。
- `src/compiler/parser/fstring_nodes.ark` に `fstring_format_debug_call()` を追加。
- 新規 fixture: `tests/fixtures/string_interp/fstring_debug_specifier.ark`

### Phase 2 — ジェネリック assert_eq
- `std/test/mod.ark` に `assert_eq<T: Eq + Debug>(actual: T, expected: T)` を追加。
- `std/prelude.ark` のレガシー `assert_eq` を削除/置換。
- 新規 fixture: `tests/fixtures/stdlib_test/assert_eq_generic.ark`

### Phase 3 — コンテナ helper 補完
- `std/text/fmt.ark` の `debug_format_vec_*` / `debug_format_option_*` / `debug_format_result_*` があれば維持。
- 不足があれば追加。

### Phase 4 — 検証
- `python3 scripts/manager.py selfhost build-compiler`
- `arukellt run tests/fixtures/string_interp/fstring_debug_specifier.ark`
- `arukellt run tests/fixtures/stdlib_test/assert_eq_generic.ark`
- `python3 scripts/manager.py verify quick`

## 4. 作業レーン・並列可否

- #704 / #705 と並列可能。影響ファイルは異なる。
- f-string パーサー変更は他の parser 変更レーンと競合する可能性がある。

## 5. 検証コマンド

```bash
python3 scripts/manager.py selfhost build-compiler
arukellt run tests/fixtures/string_interp/fstring_debug_specifier.ark
arukellt run tests/fixtures/stdlib_test/assert_eq_generic.ark
python3 scripts/manager.py verify fixtures
python3 scripts/manager.py verify quick
```

## 6. リスク

- マクロレス設計なので、f-string パーサーに `:` 構文を追加する影響範囲が広い。
- `assert_eq<T: Eq + Debug>` は `Eq` 汎用化前にスカラー型に限定する必要がある。
- `std/prelude.ark` の変更は既存コードに影響。

## 7. 進捗更新規則

- Phase 1, 2 完了後に fixture と共に commit。
- `std/prelude.ark` 変更は他の stdlib レーンとのマージ時に親オーケストレータが調整。