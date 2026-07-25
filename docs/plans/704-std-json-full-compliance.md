# #704 — std::json Full JSON RFC 8259 Compliance クローズ計画

ステータス: 完了（2026-07-26, lane `wave/704-json-full`）  
親 issue: [#704](../../issues/done/704-std-json-full-compliance.md)  
前提: #606 done  
担当 subagent lane: `wave/704-json-full`  
作業 worktree: `.worktrees/wave-704-json-full`  
作成日: 2026-07-25

## 1. 現状とゴール

- `std/json.ark`（ストリーミング）と `std/json/parser.ark`（DOM）が存在。
- DAP / LSP のローカル JSON 実装は削除済み。呼び出し元は `std::json` 直結。
- Unicode surrogate pair・不正エスケープ拒否・代表的 RFC 8259 +/- fixture を追加済み。

## 2. 完了証拠

- rfc8259 fixtures: 18（hosted smoke 18/18 PASS）
- `verify lane` PASS
- `selfhost build-compiler` PASS
- issue: `issues/done/704-std-json-full-compliance.md`

## 3. フェーズ（結果）

### Phase 1 — LSP 層移行 — done
- 深い実装と thin facade を削除。呼び出し元を `std::json` に置換。
- DAP `json_*` thin facade も同様に削除。

### Phase 2 — 文法機能追加 — done
- surrogate pair / UTF-8 BMP、未知エスケープ・未終端・trailing comma 拒否。

### Phase 3 — RFC 8259 fixture 追加 — done
- `tests/fixtures/stdlib_json/rfc8259/` に代表的な正・負 fixture。

### Phase 4 — 検証 — lane done
- `verify lane` PASS。merge 時に `verify quick` / `selfhost parity` を orchestrator が実行。

## 4. 残差（非ブロッキング）

- 外部 JSONTestSuite 全体の取り込みは未実施。
- DOM 数値経路の `parse_f64` 環境依存は既存問題として残る。
