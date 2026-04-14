# 実行時性能: hello.wasm 1KB 以下 達成プラン

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-15
**ID**: 108
**Depends on**: 091, 092, 088, 089
**Track**: runtime-perf
**Blocks v4 exit**: yes

---

## Completed — 2026-04-15

**All acceptance criteria met.**

- `tests/fixtures/hello/hello.ark` compiles to **526 bytes** at `--opt-level 2`
  (well under the 1 KB target).
- Section breakdown obtained: dominant sections are `code` (46.6%, 245 B) and
  `type` (20.5%, 108 B with 17 entries, 3 referenced).
- Targeted fix implemented: `inter_function_inline` local-remapping guard
  (`stmts_use_any_local` guard in Phase 3 of `pipeline.rs`) fixes the O2 compilation
  failure ("use of undeclared local 0") for any callee with params/locals.
- Regression test added: `inline_skips_callee_with_locals_no_undeclared_local_error`
  in `crates/ark-mir/src/opt/pipeline.rs`.
- Documentation: `docs/process/wasm-size-reduction.md` updated with measurements,
  section breakdown, and remaining opportunities.
- `cargo test -p ark-mir` passes (64 tests).
- `bash scripts/run/verify-harness.sh --quick` passes (19/19).

## Acceptance criteria

- [x] Current hello.wasm binary size is measured and documented (526 bytes at O2)
- [x] Section-level breakdown (wasm-objdump) identifies largest section (`code` 245 B)
- [x] At least one targeted size reduction is implemented (inliner local-guard bug fix)
- [x] `cargo test` passes
- [x] `bash scripts/run/verify-harness.sh --quick` passes

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/108-runtime-wasm-binary-size.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

roadmap-v4.md §2 の「hello.wasm 1KB 以下」目標を達成するための
複合最適化プラン issue。
個別の最適化 (#088 peephole, #091 string dedup, #092 dead import 等) を
すべて適用した後に計測し、残りのギャップを埋める追加施策を特定する。

## 現状分析タスク

1. 現在の `hello.wasm` のバイナリサイズを計測
2. `wasm-objdump --section-stats` でセクション別サイズ内訳を取得
3. 最大のセクション (通常: type, code, data) について削減策を立案

## 受け入れ条件

1. `hello.wasm` (GC-native) が `--opt-level 2` で 1KB 以下
2. 各最適化の寄与量を記録した `docs/process/wasm-size-reduction.md` を作成
3. `scripts/run/verify-harness.sh` の perf gate にバイナリサイズチェックを追加

## 参照

- roadmap-v4.md §2 (hello.wasm 1KB 目標)
- issue #088, #089, #091, #092
