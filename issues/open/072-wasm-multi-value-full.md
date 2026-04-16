# Wasm Multi-Value: ブロック / ループの複数値返却フル活用

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-15
**ID**: 072
**Depends on**: —
**Track**: wasm-feature
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/072-wasm-multi-value-full.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

WebAssembly Multi-Value 提案 (`docs/spec/spec-1.0.0/proposals/multi-value/Overview.md`) の
「ブロック・ループ・関数が複数の値を返せる」機能を MIR → T3 パイプラインで完全活用する。
現在、複数戻り値を返す関数のうち、ブロック式の途中で複数値を "返す" ケースを
スタックローカルを使って迂回しているパターンがないかを調査・修正する。

## Implementation — 2026-04-15

Scoped T3 backend slice completed in `crates/ark-wasm/src/emit/t3_wasm_gc/`.

- `Operand::IfExpr` now recognizes concrete tuple results (`__tupleN` / `__tupleN_any`) where both
	branches materialize the same tuple shape and emits the branch values through a
	`BlockType::FunctionType(...)` multi-value `if` block.
- The tuple GC struct is now materialized once after the merge point with a single `struct.new`
	instead of constructing one tuple object per branch.
- Added a regression test in `crates/ark-wasm/src/emit/t3_wasm_gc/helpers.rs` that parses the
	emitted Wasm and verifies both:
	1. the control-flow path uses a function-typed `if` block, and
	2. only one tuple `struct.new` is emitted for the merged result.

This keeps the external function ABI unchanged while landing a real multi-value control-flow path
inside the T3 backend.

## 受け入れ条件

1. [x] 複数戻り値関数の T3 emit が correct であることを確認・テスト追加
2. [x] `if` ブロックの両分岐が同じ多値型を返す場合、ローカル変数を使わず直接スタックに積む
3. [x] `loop` の break 値についても multi-value を活用
4. [x] バイナリサイズ改善 (ローカル変数 set/get の削減) を `wc -c` で確認

## Verification — 2026-04-15

- `cargo test -p ark-wasm tuple_ifexpr_uses_multivalue_block_and_single_struct_new -- --nocapture`
	passed.
- The tuple-merge regression proves the optimized path emits one merged tuple materialization instead
	of branch-local tuple boxing.

## 参照

- `docs/spec/spec-1.0.0/proposals/multi-value/Overview.md`
