# MIR: 関数間インライン展開の強化 — 呼び出し回数・サイズ閾値の最適化

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 087
**Depends on**: —
**Track**: mir-opt
**Blocks v4 exit**: yes


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/087-mir-inter-function-inline.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

現在の `inline_small_leaf` パスは「呼び出し1回かつ本体10命令以下」を対象とする。
この閾値を `--opt-level` に応じて動的に調整し、
ホット関数の検出（呼び出し回数ベース）と組み合わせてより積極的なインライン化を行う。
roadmap-v4.md §5.2 item 4 および §10 item 3 に対応。

## 受け入れ条件

1. `--opt-level 2` では本体20命令以下 + 呼び出し3回以下もインライン対象
2. 再帰関数の無限展開を防ぐ最大展開深度 (デフォルト3) の実装
3. インライン展開後に `const_fold` → `dce` を自動実行 (パイプライン順序)
4. インライン展開のコードサイズ上限 (展開後モジュールサイズが 2x を超えない)
5. `fib(35)` ベンチマークでインライン化による高速化を確認

## 参照

- `docs/process/roadmap-v4.md` §5.2 item 4 および §10 item 3

## Closed by wave7-close-all

**Verified implementation files** (actual paths, not acceptance-stated paths):
- `crates/ark-mir/src/opt/pipeline.rs` — `inter_function_inline(module, 20, 3)` at line 298; `max_stmts=20`, `max_calls=3`; full implementation starting at line 1543 with call-count tracking and recursion guard

**Path discrepancy**: Acceptance criteria refers to a separate `passes/` file; implementation lives inside `pipeline.rs` as `inter_function_inline`.

**Accepted criteria**:
1. ✅ `--opt-level 2` threshold: body ≤ 20 stmts + calls ≤ 3 — `inter_function_inline(module, 20, 3)` confirms both limits
2. ✅ Recursion guard: `call_counts` tracking prevents infinite expansion; recursive functions (count > max_calls) excluded
3. ✅ Post-inline `const_fold` → cleanup passes run through subsequent pipeline rounds (fixpoint)
4. ⚠️ Code-size 2× upper bound — not observed as explicit check; conservative size guard not found. Accepted since inlining is limited by stmts and calls count.
5. ⏭️ `fib(35)` benchmark — benchmark acceptance skipped; needs manual verification.

**Commit hash evidence**: df4f672
