# #824 — Early body lowering (worklist; design first) クローズ計画

ステータス: **完了（wontfix / defer, 2026-07-26）** — Phase 1–2 のみ。実装なし  
親 issue: [#824](../../issues/done/824-early-body-lowering-worklist.md)  
前提: #823, #829, #730 done  
担当 subagent lane: `wave/824-early-body`  
作業 worktree: `.worktrees/wave-824-early-body`  
作成日: 2026-07-25  
完了日: 2026-07-26

## 1. 現状とゴール

- #829 の計測結果により、現在の支配相は `emit.code.locals`（半減済み）と
  `lower.reachability` であり、`decl_emit` は支配的ではない。
- 目標: `decl_emit` が壁時間の過半を占めることが証明された場合のみ、early body
  lowering を実装する。
- **判定結果: ゲート未達 → #824 を wontfix close。実装しない。**

## 2. 前提・依存

- #823（quadratic MIR vector rebuilds）done。
- #829（phase re-profile）done。
- `KEEP_CLOCK` / `--time` 機能が動作すること。

## 3. フェーズと完了条件

### Phase 1 — 計測 ✅

- `ARUKELLT_OVERLAY_KEEP_CLOCK=1` 経路で s2-runtime を用意したうえで
  `build_clock_capable_s2` → `arukellt-s2-clock.wasm`
- KEEP_CLOCK + flat-src で
  `compile src/compiler/main.ark --target wasm32-gc --wasi-version wasi-p2 --time`
- Receipt:
  `docs/research/receipts/824-early-body-decl-emit-defer-receipt.json`

（注: 計画文面の `build-compiler` だけでは `arukellt-s2-clock.wasm` は出ない。
clock artifact は `scripts.selfhost.checks.build_clock_capable_s2` が正。）

### Phase 2 — 判定 ✅ → **defer / wontfix**

実装ゲート: `decl_emit` ≥ total の 35% **かつ** 第 2 位の 1.5 倍以上。

| ソース | decl_emit | total | share | 第2位相当 | 比 |
|---|---:|---:|---:|---|---:|
| Lane L5 2026-07-26 | 11311 ms | 102131 ms | **11.1%** | emit 41175 ms | 0.27× |
| #829 after（静穏） | 5601 ms | 31343 ms | **17.9%** | reachability 13180 ms | 0.43× |

両ゲートとも失敗。支配相は `emit` / `lower.reachability`。

### Phase 3–5 — 実装 / 検証

**スキップ。** 設計ロックは issue 本文に残すが、コード変更なし。

## 4. 作業レーン・並列可否

- #826 とは独立（本レーンは計測のみ）。
- #807 とは独立。

## 5. 検証コマンド（計測）

```bash
# host (stub) then clock artifact
ARUKELLT_OVERLAY_KEEP_CLOCK=1 python3 scripts/manager.py selfhost build-compiler
python3 -c 'from pathlib import Path; from scripts.selfhost.checks import build_clock_capable_s2; print(build_clock_capable_s2(Path(".").resolve()))'
# prefer isolated probe when no concurrent main.ark:
python3 scripts/debug/latency_rss_phase_probe.py
# if REFUSE: same argv as probe under documented concurrent load
```

## 6. リスク（クローズ時点）

- 実装しても `decl_emit` 節約は壁時間の ~10–18% 帯が上限見込みで、
  reachability / emit を先に攻める方が効く。
- 将来 `decl_emit` が支配的になったら issue を再オープンし設計ロックに従う。

## 7. 進捗更新規則

- Phase 2 の判定を issue 本文に記録済み。
- wontfix 理由と receipt パスを issue / 本計画に残済み。
