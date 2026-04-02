# Parser parser.rs (2003行) をサブモジュールに分割

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 132
**Depends on**: —
**Track**: code-structure
**Blocks v4 exit**: no

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
