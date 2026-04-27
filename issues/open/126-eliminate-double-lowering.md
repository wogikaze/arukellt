---
Status: open
Created: 2026-03-28
Updated: 2026-04-03
ID: 126
Track: pipeline-refactor
Depends on: 125
Orchestration class: blocked-by-upstream
Orchestration upstream: None
Blocks v4 exit: "no (compile-time optimization)"
Implementation target: Trusted-base compiler route named below while it remains active; do not turn this into #529 selfhost-retirement work.
Status note: Trusted-base compiler cleanup blocked on #125. Keep separate from #285/#508/#529 legacy removal and from #099 selfhost frontend design.
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
---

`compile()` が `MirSelection: ":Legacy` のときは `corehir_mir` は使われない (逆も同様)。"
2. `cargo bench` でコンパイル時間が短縮されること (v4 目標: 50ms / 500ms)
- v4 コンパイル時間目標: hello.ark ≤ 50ms、arukellt.ark ≤ 500ms
- `crates/ark-driver/src/session.rs: "296-370` (run_frontend — 二重 lower)"
# `run_frontend()` の二重 lower を解消 (遅延 lower)



---

## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/126-eliminate-double-lowering.md` — incorrect directory for an open issue.


## Summary

`ark-driver/src/session.rs` の `run_frontend()` は毎回 Legacy MIR と CoreHIR MIR の
**両方** を生成してから呼び出し元に返している。

```rust
// run_frontend() 内
let mut legacy_mir = lower_legacy_only(&resolved.module, &checker, ...);  // always
let corehir_mir = if corehir_valid {
    lower_check_output_to_mir(&resolved.module, &core_hir, ...)  // always
} else { ... };
```

`compile()` が `MirSelection::Legacy` のときは `corehir_mir` は使われない (逆も同様)。
不要な lower によりコンパイル時間が最大2倍になっている。

## Responsibility split — 2026-04-22

This issue is the performance/cleanup follow-up to #125 after the trusted-base
compiler default path is corrected. It should not be grouped with #285/#508/#529
legacy removal or #099 selfhost frontend design work for dispatch decisions.

## 受け入れ条件

1. `run_frontend()` をリファクタして選択された `MirSelection` に応じてのみ lower を実行
   - あるいは `lower_mir_selected()` の実装を選択済み MIR だけを生成するよう変更
2. `cargo bench` でコンパイル時間が短縮されること (v4 目標: 50ms / 500ms)
3. `python scripts/manager.py verify` が status 0 で終了
4. parity テスト (`compare_mir_paths`) は引き続き明示的に両方の lower を実行

## 背景

- v4 コンパイル時間目標: hello.ark ≤ 50ms、arukellt.ark ≤ 500ms
- `compare_mir_paths()` は MIR 比較のため両パスが必要 (影響なし)
- Issue #125 (CoreHIR デフォルト化) が完了後、Legacy lower は `parity test` 専用に限定

## 参照

- `crates/ark-driver/src/session.rs:296-370` (run_frontend — 二重 lower)
- `docs/process/roadmap-v4.md` (コンパイル時間目標)