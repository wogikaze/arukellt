# #705 — std::toml Full TOML 1.0 Compliance クローズ計画

ステータス: 完了  
親 issue: [#705](../../issues/done/705-std-toml-full-compliance.md)  
前提: #606 done  
担当 subagent lane: `wave/705-toml-full`  
作業 worktree: `.worktrees/wave-705-toml-full`  
作成日: 2026-07-25

## 1. 現状とゴール

- `std/toml/parser.ark` は TOML 1.0 コア文法を実装済み。
- `tests/fixtures/stdlib_toml/` に 80+ fixture あり。
- コンパイラ内部 (`src/compiler/main/project_run.ark`, `src/compiler/lint/config.ark`) に残存ヘルパーあり。
- 目標:
  - 内部 TOML ヘルパーを `std::toml` に移行。
  - `src/compiler/main/script_toml.ark` と `src/compiler/lsp/symbol_index_paths.ark` を削除。
  - `verify quick` 0 失敗。

## 2. 前提・依存

- #606 done。
- `std::toml` の API (`toml_find_toml_value`, `toml_table_keys` 等)。

## 3. フェーズと完了条件

### Phase 1 — ヘルパー統合
- `src/compiler/main/project_run.ark` の `find_toml_value` を削除し、`std::toml` に置換。
- `src/compiler/lint/config.ark` の `toml_find_raw_value` / `parse_toml_string_array` を `std::toml` に移動。

### Phase 2 — 呼び出し元更新
- 以下で `symbol_index_paths::` / `script_toml::` プレフィックスを `toml::` に変更:
  - `src/compiler/main/script.ark`
  - `src/compiler/main/manifest_doc_parse.ark`
  - `src/compiler/lsp/symbol_index_stdlib.ark`
  - `src/compiler/lsp/symbol_index_project.ark`
  - `src/compiler/loader/wit_manifest.ark`
  - `src/compiler/loader/registry_config.ark`
  - `src/compiler/lint/config.ark`

### Phase 3 — モジュール削除
- `src/compiler/main/script_toml.ark` 削除（完了）。
- `symbol_index_paths.ark` から TOML ヘルパーを除去（完了）。ファイル本体は URI/path
  ヘルパー専用として残す（#704 の LSP JSON 移行と衝突させない）。
  パスヘルパーの別ファイルへの移設は本 issue の非ゴール。

### Phase 4 — 検証
- `toml_get` / `toml_table_keys` のネストテーブルアクセスを確認。
- 負の fixture (`toml_err_*.ark`) で不正 TOML が拒否されることを確認。
- `python3 scripts/manager.py verify quick`。

## 4. 作業レーン・並列可否

- #704 / #696 と並列可能。
- `#704` も `src/compiler/lsp/` を変更するため、マージ時に親オーケストレータが競合確認。

## 5. 検証コマンド

```bash
python3 scripts/manager.py verify fixtures
python3 scripts/manager.py verify quick
```

## 6. リスク

- 多数のコンパイラモジュールが内部 TOML ヘルパーに依存。
- `toml_find_raw_value` は配列解析用の特殊ケース。
- ユーザー可達 API ではないため破壊的変更の影響は小さい。

## 7. 進捗更新規則

- Phase 2 完了後に `verify quick` を実行。
- 全 Phase 完了後に issue を `issues/done/` へ移動。