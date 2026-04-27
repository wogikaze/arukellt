---
Status: done
Created: 2026-03-29
Updated: 2026-04-15
ID: 140
Track: benchmark
Depends on: 149
Orchestration class: implementation-ready
Blocks v1 exit: no
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
---

# ベンチ統合: `mise bench` 1コマンド導線と subcommand 整理
- [x] `mise bench: "quick`、`mise bench:compare`、`mise bench:update-baseline`、`mise bench:ci` を用意する"
# ベンチ統合: `mise bench` 1コマンド導線と subcommand 整理

---

## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/140-bench-one-command-workflow.md` — incorrect directory for an open issue.


## Summary

既存の `benchmarks/run_benchmarks.sh`、`scripts/compare-benchmarks.sh`、`scripts/check/perf-gate.sh` が分散しているため、
ローカル計測・比較・ベースライン更新を `mise bench` 系の 1 コマンド導線に統合する。
「まず何を叩けばよいか」を固定し、ベンチ運用の心理的コストを下げる。

## 受け入れ条件

- [x] `mise bench` で release build + compile/runtime/size/memory の標準計測が走る
- [x] `mise bench:quick`、`mise bench:compare`、`mise bench:update-baseline`、`mise bench:ci` を用意する
- [x] `hyperfine` / `wasmtime` など任意依存が欠ける場合に、何が skip されたか明示する
- [x] `benchmarks/README.md` と `docs/process/benchmark-results.md` に新しい導線を反映する

## 実装タスク

1. `mise.toml` に benchmark 系 task を追加する
2. 既存 shell script の入出力形式を揃えて orchestration しやすくする
3. quick/local/ci で同じコマンド体系を保ち、違いは preset で吸収する

## 参照

- `benchmarks/run_benchmarks.sh`
- `scripts/compare-benchmarks.sh`
- `scripts/check/perf-gate.sh`
- `benchmarks/README.md`