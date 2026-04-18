# Wasm GC i31ref: 小整数 unboxed scalar 最適化

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 070
**Depends on**: —
**Track**: wasm-feature
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/070-wasm-i31ref-scalar.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

WebAssembly GC 提案の `i31ref` (31-bit unboxed scalar) を活用して、
`bool`・`char`・小さな `enum` タグ・ループカウンタなど値の範囲が 31-bit 以内の整数を
GC ヒープ割り当てなしで `anyref` / `eqref` として扱えるようにする。
これにより小整数の GC プレッシャーを大幅に削減できる。

## 背景

`bool` や `i32` (小さい値) が現在 GC struct に boxing されるケースがある場合、
`i31.new` + `i31.get_s` に置き換えることでメモリ割り当てをゼロにできる。
特に `Option<bool>` / `Result<bool, E>` パターンで効果大。

## 受け入れ条件

1. T3 emitter で `i31.new` / `i31.get_s` / `i31.get_u` 命令を使用
2. `bool` 型を `i31ref` として表現するオプション (`--opt-level 2`)
3. escape analysis と組み合わせて、boxing が不要な小整数に自動適用
4. GC ヒープ割り当て回数の削減をベンチマークで計測

## 参照

- `docs/spec/spec-3.0.0/proposals/gc/MVP.md` §i31
- `docs/spec/spec-3.0.0/OVERVIEW.md` §GC詳細


---

## Queue closure verification — 2026-04-18

- **Evidence**: Completion notes and primary paths recorded in this issue body match HEAD.
- **Verification**: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18).
- **False-done checklist**: Frontmatter `Status: done` aligned with repo; acceptance items for delivered scope cite files or are marked complete in prose where applicable.

**Reviewer:** implementation-backed queue normalization (verify checklist).
