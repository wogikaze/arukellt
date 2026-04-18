# ベンチマーク比較: C/Rust/Go/Grain との自動比較スクリプト

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 112
**Depends on**: 109
**Track**: benchmark
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v4 exit**: yes

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/112-bench-compare-langs.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

roadmap-v4.md §5.4 で選定した比較言語 (C gcc-O2 / Rust --release / Go / Grain) との
ベンチマーク比較を自動化するスクリプトを `scripts/compare-benchmarks.sh` として追加する。

## 受け入れ条件

1. `scripts/compare-benchmarks.sh` が各言語のバイナリをビルドして hyperfine で計測
2. 結果を `docs/process/benchmark-results.md` に Markdown テーブルとして出力
3. C 比 (fib: 1.5x 以内, vec-ops: 2.0x 以内) の合否を自動判定
4. Grain (Wasm-native GC 言語) との比較で Arukellt の優位性を確認

## 参照

- roadmap-v4.md §5.4

## Prior “closed” note (superseded)

An earlier edit marked this issue closed while **`Status` remained `open`** and acceptance was not evidenced in-repo. The **2026-04-03 reopen** (above) is authoritative. The bullet list below is historical context only (partial shell fixes landed in b9db40c); remaining acceptance is tracked against the checklist in the next section.

- Fixed `--compare-lang VALUE` (space-separated) argument parsing
- Fixed `local` keyword used outside function in compare-lang timing loop
- Added graceful toolchain availability checks (cc/gcc/rustc/go)
- C references (fib.c, binary_tree.c) and README comparison section were already present from #109

## Acceptance vs repo (audit)

| # | Criterion | Met in repo? | Notes |
|---|-----------|--------------|-------|
| 1 | `scripts/compare-benchmarks.sh` builds each ref lang and times with hyperfine when installed | **Yes** | `run-benchmarks.sh --compare-lang`; falls back to shell timer if hyperfine missing |
| 2 | Markdown table written to `docs/process/benchmark-results.md` | **Yes** | Embedded between `<!-- arukellt:cross-lang-compare:start/end -->` via `--compare-write-md` (default from `compare-benchmarks.sh`) |
| 3 | C-ratio gates fib ≤1.5×, vec_ops ≤2.0× vs C | **Yes** | `--compare-c-ratio-gate` (default from `compare-benchmarks.sh`); **skipped** if `benchmarks/*.c` or `cc` missing — no fake fail |
| 4 | Grain comparison | **No (STOP_IF)** | No `benchmarks/*.grain`, no `grain` CLI hook in runner; needs external Grain toolchain + sources |

**Status**: Remains **open** until Grain slice is implemented or explicitly descoped with ADR/issue update.
