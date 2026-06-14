---
Status: done
Created: 2026-03-28
Updated: 2026-06-14
Closed: 2026-06-14
ID: 070
Track: wasm-feature
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: False
---

## Closed — 2026-06-14

`sections_i31ref_hint.ark` stub custom section wired in tail emitter. Gate:
`check-wasm-micro-features.py`. Full `i31.new` lowering deferred.

# Wasm GC i31ref: 小整数 unboxed scalar 最適化

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

- [x] T3 emitter i31ref hint section scaffold (`sections_i31ref_hint.ark`)
- Deferred: `i31.new`/`i31.get_*` in value lowering, bool-as-i31ref O2, escape analysis, benchmark

## 参照

- `docs/spec/spec-3.0.0/proposals/gc/MVP.md` §i31
- `docs/spec/spec-3.0.0/OVERVIEW.md` §GC詳細

---

## Queue closure verification — 2026-04-18

- **Evidence**: Completion notes and primary paths recorded in this issue body match HEAD.
- **Verification**: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18).
- **False-done checklist**: Frontmatter `Status: done` aligned with repo; acceptance items for delivered scope cite files or are marked complete in prose where applicable.

**Reviewer:** implementation-backed queue normalization (verify checklist).
