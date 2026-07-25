# #809 — WAT roundtrip failure クローズ計画

ステータス: 完了  
親 issue: [#809](../../issues/done/809-wat-roundtrip-failure.md)  
担当 subagent lane: `wave/809-wat-roundtrip`  
作業 worktree: `.worktrees/wave-809-wat-roundtrip`  
作成日: 2026-07-25  
完了日: 2026-07-26

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
- 推定原因（当初）は `src/compiler/wasm/` 内の文字列 / カスタムセクション / 型セクションエンコーディング。
- **実原因**: match パターン解析・低下（`types::WitType::Bool` パスパターン未解析、裸 unit variant の catchall 誤認）。

## 3. フェーズと完了条件

### Phase 1 — 最小再現 — 完了
- `scripts/run/arukellt-selfhost.sh compile --target wasm32 tests/fixtures/stdlib_component/canonical_list.ark -o /tmp/test.wasm`
- `wasm-tools print /tmp/test.wasm` / `wasm-tools validate /tmp/test.wasm` でエラーメッセージを取得。

### Phase 2 — 根本原因特定 — 完了
- 失敗は主に `stdlib_wit/wit_print.ark` → `std/wit/printer.ark` の
  module-qualified match patterns。
- 追加で bare `None`/`Some`/`Ok`/`Err` が catchall 扱いになり orphan `else`。

### Phase 3 — 修正 — 完了
- Parser / MIR match lowering / exhaustiveness（`4e07e2a6`）。

### Phase 4 — 検証 — 完了
- `bash scripts/run/wat-roundtrip.sh` → PASS=3152 FAIL=0 SKIP=39
- `python3 scripts/manager.py verify lane` → PASS

## 4. 作業レーン・並列可否

- 実変更は `src/compiler/parser/` + `mir/lower/` + `typechecker/`。
- `#809` は独立レーンで完了。

## 5. 検証コマンド

```bash
bash scripts/run/wat-roundtrip.sh
python3 scripts/manager.py verify lane
```

## 6. リスク

- （クローズ時）残存なし。全 run: fixture の WAT roundtrip が緑。

## 7. 進捗更新規則

- Phase 2 で根本原因を issue 本文に追記済み。
- クローズ時に `release-guarantees.toml` の `check_wat_roundtrip` を pass へ更新。
