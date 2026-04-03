# 基盤: benchmark schema・命名・実行モードの標準化

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-04-03
**ID**: 149
**Depends on**: —
**Track**: benchmark
**Blocks v1 exit**: no


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/149-benchmark-governance-and-schema.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

ベンチ関連スクリプトが増える前に、結果 schema・fixture 命名・quick/full/ci モード・baseline 更新ルールを標準化する。
後続 issue が同じ言葉で会話できる土台を先に作る。

## 受け入れ条件

1. benchmark result schema (compile/runtime/size/memory/metadata) を定義する
2. fixture 命名規約と tag/taxonomy の置き場を決める
3. `quick` / `full` / `compare` / `ci` / `update-baseline` の意味を明文化する
4. `benchmarks/README.md` と `docs/process/benchmark-plan.md` に標準ルールを反映する

## 実装タスク

1. 既存 benchmark script の出力項目を棚卸しする
2. 最小共通 schema と optional field を定義する
3. 新規 benchmark issue が従うテンプレートを決める

## 参照

- `benchmarks/README.md`
- `docs/process/benchmark-plan.md`
- `scripts/check/perf-gate.sh`
- `scripts/compare-benchmarks.sh`

## Close Evidence

- `benchmarks/README.md`: updated with conceptual field reference (`compile_time_ms`, `runtime_ms`, `wasm_size_bytes`, `peak_memory_bytes`), `<suite>/<name>.<ext>` fixture naming convention, and tag/taxonomy guidance.
- `docs/process/benchmark-plan.md`: created with run-mode definitions (`quick` smoke <30s, `full`, `compare`, `ci` PR gate, `update-baseline`), regression thresholds, and baseline update rules (when/who/how).
- `bash scripts/run/verify-harness.sh --quick`: **19/19 PASS**
- `python3 scripts/check/check-docs-consistency.py`: **PASS**
- No Rust or `.ark` source files modified.
