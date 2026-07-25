# #809 — WAT roundtrip failure クローズ計画

ステータス: 計画  
親 issue: [#809](../../issues/open/809-wat-roundtrip-failure.md)  
担当 subagent lane: `wave/809-wat-roundtrip`  
作業 worktree: `.worktrees/wave-809-wat-roundtrip`  
作成日: 2026-07-25

## 1. 現状とゴール

- `verify full` の `wat_roundtrip` チェックで 6 件の失敗（3 fixture × 2 target）。
- 対象 fixture:
  - `stdlib_component/canonical_list.ark`
  - `stdlib_component/canonical_string.ark`
  - `stdlib_wit/wit_print.ark`
- すべて `wasm-tools print` が ill-formed binary と判定。
- 目標: これらの fixture が `wasm-tools print` とその後の `wasm-tools validate` を通過。

## 2. 前提・依存

- なし。
- 推定原因は `src/compiler/wasm/` 内の文字列 / カスタムセクション / 型セクションエンコーディング。

## 3. フェーズと完了条件

### Phase 1 — 最小再現
- `scripts/run/arukellt-selfhost.sh compile --target wasm32 tests/fixtures/stdlib_component/canonical_list.ark -o /tmp/test.wasm`
- `wasm-tools print /tmp/test.wasm` / `wasm-tools validate /tmp/test.wasm` でエラーメッセージを取得。

### Phase 2 — 根本原因特定
- 3 つの失敗 fixture の共通点を調査:
  - Canonical ABI helpers (`std/component/canonical.ark`)
  - WIT printer (`std/wit/printer.ark`)
- 問題箇所を特定:
  - `src/compiler/wasm/strings.ark`
  - `src/compiler/wasm/wasm_sections.ark`
  - `src/compiler/wasm/sections_types.ark`

### Phase 3 — 修正
- 該当するエンコーディングバグを修正。

### Phase 4 — 検証
- `bash scripts/run/wat-roundtrip.sh`
- `python3 scripts/manager.py verify full`

## 4. 作業レーン・並列可否

- `src/compiler/wasm/` に集中。他レーンと競合が少ない。
- `#809` は小規模で独立。

## 5. 検証コマンド

```bash
scripts/run/arukellt-selfhost.sh compile --target wasm32 tests/fixtures/stdlib_component/canonical_list.ark -o /tmp/test.wasm
wasm-tools print /tmp/test.wasm -o /tmp/test.wat
wasm-tools validate /tmp/test.wasm
bash scripts/run/wat-roundtrip.sh
python3 scripts/manager.py verify full
```

## 6. リスク

- 修正中に同種のエンコーディング不備が他の箇所で発見される。
- 特定 fixture パターンに限られた問題だが、根本的な emitter バグの可能性。

## 7. 進捗更新規則

- Phase 2 で根本原因を issue 本文に追記。
- 修正後に `verify full` の `wat_roundtrip` 結果を記録。