---
Status: done
Created: 2026-03-28
Updated: 2026-04-03
ID: 109
Track: benchmark
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: True
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
# ベンチマーク: fib / binary_tree / vec_ops / string_concat / json_parse スイート
---
# ベンチマーク: fib / binary_tree / vec_ops / string_concat / json_parse スイート

---

## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: done` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/109-bench-suite.md` — incorrect directory for an open issue.


## Summary

roadmap-v4.md §6 item 7 で要求されているベンチマークスイートを `benchmarks/` に構築する。
比較言語 (C/Rust/Go/Grain) の同等プログラムと並べて計測できる構造にする。

## ベンチマーク一覧

| 名前 | ファイル | 計測内容 |
|------|---------|---------|
| fib | `benchmarks/fib.ark` | fib(35) の実行時間 |
| binary_tree | `benchmarks/binary_tree.ark` | depth=15 の木構築・走査 |
| vec_push_pop | `benchmarks/vec_push_pop.ark` | 10万要素の push/pop |
| string_concat | `benchmarks/string_concat.ark` | 1万回 string concat |
| json_parse | `benchmarks/json_parse.ark` | 10KB JSON のパース |
| nbody | `benchmarks/nbody.ark` | 天体計算 (浮動小数点集中) |
| mandelbrot | `benchmarks/mandelbrot.ark` | フラクタル計算 |

## 受け入れ条件

1. 各ベンチマーク Ark ファイルと C/Rust 版参照実装を `benchmarks/` に配置
2. `scripts/run/run-benchmarks.sh --compare-lang c,rust,go` で一括計測
3. `benchmarks/README.md` に計測方法・比較対象・結果表を記載
4. hyperfine 3回中央値で計測

## 参照

- roadmap-v4.md §5.4 および §6 item 7