# #824 — Early body lowering (worklist; design first) クローズ計画

ステータス: 計画（design phase）  
親 issue: [#824](../../issues/open/824-early-body-lowering-worklist.md)  
前提: #823, #829, #730 done  
担当 subagent lane: `wave/824-early-body`  
作業 worktree: `.worktrees/wave-824-early-body`  
作成日: 2026-07-25

## 1. 現状とゴール

- #829 の計測結果により、現在の支配相は `emit.code.locals` と `lower.reachability` であり、`decl_emit` は支配的ではない。
- 目標: `decl_emit` が wall time の過半を占めることが証明された場合のみ、early body lowering を実装する。
- そうでない場合は #824 を close（wontfix / defer）する。

## 2. 前提・依存

- #823（quadratic MIR vector rebuilds）done。
- #829（phase re-profile）done。
- `KEEP_CLOCK` / `--time` 機能が動作すること。

## 3. フェーズと完了条件

### Phase 1 — 計測
- `ARUKELLT_OVERLAY_KEEP_CLOCK=1 python3 scripts/manager.py selfhost build-compiler`
- `python3 scripts/debug/latency_rss_phase_probe.py compile src/compiler/main.ark --target wasm32-gc --wasi-version wasi-p2 --time`
- `.build/selfhost/selfhost-latency-receipt.json` を確認し、`decl_emit` の壁時間シェアを取得。

### Phase 2 — 判定
- `decl_emit` が total の 35% 以上かつ第 2 位の 1.5 倍以上なら実装へ進む。
- そうでなければ #824 を close し、`lower.reachability` 最適化に注力。

### Phase 3 — 設計ロック（実装する場合）
- ワークキューオーダー: `main` → `_start` → exports → WIT → conservative set → `FunctionId.raw` 昇順。
- ルートシードを明確化。
- Never-lowered bodies は署名/FunctionId/layout/type メタデータは全登録。
- Post-MIR prune を安全網として維持。
- Stage-2 overlay keep 契約を維持。

### Phase 4 — 実装（設計ロック後）
- `src/compiler/mir/lower/entry*.ark` / `src/compiler/mir/reachability*.ark` を改修。
- `FunctionId → body-lowered?` 状態を post-MIR map と分離。

### Phase 5 — 検証
- `python3 scripts/manager.py verify quick`
- `python3 scripts/check/check-mir-reachability-bfs.py`

## 4. 作業レーン・並列可否

- #826 とは `MirModule` / `MirFunction` 構造体変更で競合する可能性がある。
- #807 とは独立。

## 5. 検証コマンド

```bash
ARUKELLT_OVERLAY_KEEP_CLOCK=1 python3 scripts/manager.py selfhost build-compiler
python3 scripts/debug/latency_rss_phase_probe.py compile src/compiler/main.ark --target wasm32-gc --wasi-version wasi-p2 --time
cat .build/selfhost/selfhost-latency-receipt.json
python3 scripts/manager.py verify quick
```

## 6. リスク

- `decl_emit` が支配的でない場合、実装しても大きな効果がない。
- ルート漏れによる不正な枝刈り。
- 決定性違反（HashMap 反復順）。
- Stage-2 overlay 回帰。

## 7. 進捗更新規則

- Phase 2 の判定を issue 本文に記録。
- 実装しない場合は wontfix ラベルと理由を issue に残す。