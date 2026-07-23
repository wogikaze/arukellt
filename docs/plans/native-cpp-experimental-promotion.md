# native-cpp selfhost executor experimental 昇格計画

Status: active  
Owner: native-cpp / #833  
Created: 2026-07-23

## 目標

strict command（override なし）が成功すること:

```bash
python3 scripts/manager.py selfhost native-executor --build
```

必須: `exit_code==0`, `high_rss_override==false`, `memory_gate_passed`, warm wall `<300s`,
peak RSS `<=2.4 GiB`, validate/equality/determinism, root liveness skip=0,
root clear 有効で GC stress PASS, strict gate 3 連続 PASS。

昇格対象は **内部 selfhost executor lane** のみ。維持するもの:

- `support_tier = scaffold`
- `run_supported = false`
- public `arukellt run --target native-cpp` / C ABI / 配布 executable は保証しない

## Critical path

```text
receipt強化
→ emitter由来の instruction effect
→ CFG-complete root liveness
→ shadow full-S3
→ fixture で root clear 有効化
→ full-S3 で段階的有効化
→ live graph / mark 時間を再計測
→ threshold 調整（必要なら typed mark / table）
→ strict gate 3 連続 PASS
→ docs/state 更新
→ experimental 昇格
```

最初から object table / allocator を全面改造しない。まず誤保持 root を減らす（#833）。

## Phases（要約）

| Phase | 内容 | PR |
|------|------|----|
| 0 | baseline 3×、receipt schema、gate 分離、GC timing | PR1 |
| 1 | emitter effect / safepoint SSOT / worklist liveness（clear はまだ off） | PR2 |
| 2 | shadow mode、unit/golden/safepoint audit、shadow full-S3 | PR2–3 |
| 3 | fixture → small-fn → 全関数で clear 有効化 | PR3–4 |
| 4 | 性能評価と分岐（threshold / iterative mark / typed scan） | PR5 |
| 5 | compiler phase owner 解放（必要時） | PR5 |
| 6 | stress / sanitizer / 3× strict | PR5–6 |
| 7 | CI / manager（override 禁止） | PR6 |
| 8 | docs / state / false-done gate 641 | PR6 |

詳細チェックリストは本会話の計画本文を正とし、実装進捗に合わせて本ファイルの checkbox を更新する。

## Baseline（Phase 0 前の既知値）

| 指標 | 値 |
|------|---:|
| arena warm wall | ~228 s |
| arena peak RSS | ~12.3 GiB |
| GC warm wall | ~480 s |
| GC peak RSS | ~1.55 GiB |
| live objects | ~13,791,754 |
| collections | 42 |

## Non-goals

- `#831` wasm32-gc 正規 fixpoint をこの昇格の blocker にしない
- public native product 完成を名乗らない
