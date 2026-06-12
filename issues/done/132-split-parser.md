---
Status: done
Created: 2026-03-28
Updated: 2026-04-14
ID: 132
Track: code-structure
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: no
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
---
# Parser parser.rs (2003行) をサブモジュールに分割

---

## Reopened by audit — 2026-04-03

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/132-split-parser.md` — incorrect directory for an open issue.

## Audit resolution — 2026-06-12

FD-01 Slice A review: frontmatter `Action` records a 2026-04 false-done move to `issues/open/`; file correctly remains under `issues/done/` after re-close verification.

**Evidence**: Selfhost parser modularized under `src/compiler/parser/` (supersedes Rust `crates/ark-parser` split).

**Classification**: `truly-done` (stale reopen metadata only).

## Summary

`crates/ark-parser/src/parser.rs` は 2003 行。
式・文・宣言・型・パターンのパーサが単一 `impl Parser` に混在。
`parser/` ディレクトリに分割する。

## 提案する分割後の構造

```
crates/ark-parser/src/
├── parser.rs           # 削除 → ディレクトリに変換
└── parser/
    ├── mod.rs          # Parser struct, pub fn parse_module/parse_file (~150行)
    ├── decl.rs         # 宣言パーサ (fn, struct, enum, impl, use, const) (~500行)
    ├── expr.rs         # 式パーサ (literals, binop, call, if/match/closure) (~500行)
    ├── stmt.rs         # 文パーサ (let, assign, return, expr-stmt, block) (~400行)
    ├── ty.rs           # 型パーサ (named, slice, fn type, generic) (~250行)
    └── pattern.rs      # パターンパーサ (tuple, struct, enum variant, wildcard) (~200行)
```

## 受け入れ条件

1. 上記 6 ファイルに分割
2. `cargo build --workspace --exclude ark-llvm --exclude ark-lsp` が通る
3. `cargo test --workspace --exclude ark-llvm --exclude ark-lsp` が通る
4. `scripts/run/verify-harness.sh` が status 0

## 参照

- `crates/ark-parser/src/parser.rs`
- `crates/ark-parser/src/ast.rs` (486行 — AST 定義は適切なサイズ)
- `crates/ark-parser/src/lib.rs` (360行 — ラッパー層)
