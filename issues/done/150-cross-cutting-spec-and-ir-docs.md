# 横断 docs: `docs/language/spec.md` 凍結版と `docs/compiler/ir-spec.md` を整備

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-04-29
**ID**: 150
**Depends on**: —
**Track**: cross-cutting
**Blocks v1 exit**: no

---

## Closure — 2026-04-29

All acceptance criteria verified:

1. ✅ `docs/language/spec.md` — comprehensive frozen v1 language spec covering lexical structure, type system (primitives, composites, generics, traits), expressions, statements, pattern matching, items, module/import system, operator precedence, stdlib API, and error codes. Each section carries a stability label (stable / provisional / experimental / unimplemented).
2. ✅ `docs/compiler/ir-spec.md` — authoritative CoreHIR/MIR reference covering: compiler pipeline overview, AST types, CoreHIR data structures (Expr/Stmt/Item/Ty), MIR data structures (MirModule/MirFunc/BasicBlock/Instruction/Value), HIR→MIR lowering rules, MIR optimization passes with pre/post conditions, MIR validation rules (invariants), and MIR→Wasm mapping.
3. ✅ `docs/compiler/pipeline.md` and `docs/language/type-system.md` both contain cross-links to the new documents.
4. ✅ `python3 scripts/check/check-docs-consistency.py` exits 0 (0 issues).
5. ✅ `bash scripts/run/verify-harness.sh --quick` passes 19/19 checks.

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/150-cross-cutting-spec-and-ir-docs.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`docs/process/roadmap-cross-cutting.md` §6.4 / §6.6 は、v5 着手前に
`docs/language/spec.md` と `docs/compiler/ir-spec.md` が揃っていることを要求している。
現状は `docs/language/syntax.md`, `docs/language/type-system.md`, `docs/compiler/pipeline.md` はあるが、
「凍結対象の言語仕様」と「CoreHIR / MIR の正規仕様書」が欠けている。

## 受け入れ条件

1. `docs/language/spec.md` が追加され、v5 着手前の凍結対象として扱う範囲が明記される
2. `docs/compiler/ir-spec.md` が追加され、CoreHIR / MIR の主要 struct / enum / invariant / phase 境界が文書化される
3. `docs/compiler/pipeline.md` と `docs/language/type-system.md` から新規文書へ辿れる
4. `scripts/run/verify-harness.sh` または docs consistency check で上記 2 ファイルの存在が検証される

## 実装タスク

1. 既存の `docs/language/*.md`, `docs/compiler/pipeline.md`, `docs/current-state.md` を棚卸しし、凍結対象と current-first 説明を分離する
2. `docs/language/spec.md` に syntax / module / import / type / control flow / error model の凍結境界を書く
3. `docs/compiler/ir-spec.md` に CoreHIR / MIR のデータ構造、phase dump 名、validation invariant、pass 前後の前提を書く
4. 関連文書からのリンクを更新し、docs drift check に組み込む

## 参照

- `docs/process/roadmap-cross-cutting.md` §6.4, §6.6
- `docs/language/syntax.md`
- `docs/language/type-system.md`
- `docs/compiler/pipeline.md`
- `docs/current-state.md`
