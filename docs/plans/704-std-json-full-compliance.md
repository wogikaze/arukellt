# #704 — std::json Full JSON RFC 8259 Compliance クローズ計画

ステータス: 進行中（L10 done: LSP thin-delegate / surrogate+strict escapes / 9 rfc8259 fixtures; remaining: delete thin facades, full JSONTestSuite, verify quick）  
親 issue: [#704](../../issues/open/704-std-json-full-compliance.md)  
前提: #606 done  
担当 subagent lane: `wave/704-json-full`  
作業 worktree: `.worktrees/wave-704-json-full`  
作成日: 2026-07-25

## 1. 現状とゴール

- `std/json.ark`（ストリーミング）と `std/json/parser.ark`（DOM）が存在。
- DAP 層は既に `std::json` に委譲済み。
- LSP 層 (`src/compiler/lsp/json_*.ark`) にローカル実装が残存。
- 目標:
  - LSP 層の JSON ヘルパーを削除し、`std::json` に完全移行。
  - Unicode surrogate pair と不正なエスケープシーケンスに対応。
  - RFC 8259 テストスイートの代表的な正・負 fixture を追加。

## 2. 前提・依存

- #606 done。
- `std::json` の DOM / ストリーミング API。

## 3. フェーズと完了条件

### Phase 1 — LSP 層移行
- 以下を削除し、呼び出し元を `std::json` に置換:
  - `src/compiler/lsp/json_escape.ark`
  - `src/compiler/lsp/json_parse_utils.ark`
  - `src/compiler/lsp/json_parse_int.ark`
  - `src/compiler/lsp/json_parse_string.ark`
  - `src/compiler/lsp/json_parse_string_escape.ark`
  - `src/compiler/lsp/json_parse_string_unicode.ark`
  - `src/compiler/lsp/json_parse_core.ark`
  - `src/compiler/lsp/json_fields.ark`
  - `src/compiler/lsp/json.ark`

### Phase 2 — 文法機能追加
- `json_decode_unicode_ascii` を拡張し、 surrogate pair を正しくデコード。
- `json_decode_escape` で未知のエスケープをエラー化。

### Phase 3 — RFC 8259 fixture 追加
- `tests/fixtures/stdlib_json/rfc8259/` に代表的な正・負 fixture を追加。

### Phase 4 — 検証
- `python3 scripts/manager.py verify fixtures`
- `python3 scripts/manager.py verify quick`
- `python3 scripts/manager.py selfhost parity`

## 4. 作業レーン・並列可否

- #705 / #696 と並列可能。
- LSP ファイルは #705 の TOML ファイルと異なるが、同じ `src/compiler/lsp/` ディレクトリを触るため注意。

## 5. 検証コマンド

```bash
python3 scripts/manager.py verify fixtures
python3 scripts/manager.py verify quick
python3 scripts/manager.py selfhost parity
```

## 6. リスク

- LSP 層の `json_escape::quote_string` 呼び出しを正しく置換する必要がある。
- コンパイラ内部コードの破壊的変更で LSP が一時的に動作しなくなる可能性。
- JSONTestSuite ファイル数が多いため、代表サブセットを選定。

## 7. 進捗更新規則

- Phase 1 完了後に LSP の smoke test を実行。
- RFC 8259 fixture 追加後に `verify fixtures` の結果を記録。