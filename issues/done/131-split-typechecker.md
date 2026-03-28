# TypeChecker checker.rs (2790行) をサブモジュールに分割

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 131
**Depends on**: —
**Track**: code-structure
**Blocks v4 exit**: no

## Summary

`crates/ark-typecheck/src/checker.rs` は 2790 行。
型推論・型チェック・ビルトイン登録・CoreHIR 検査が単一ファイルに混在している。
`typecheck/checker/` ディレクトリに分割する。

## 提案する分割後の構造

```
crates/ark-typecheck/src/
├── checker.rs           # 削除 → ディレクトリに変換
└── checker/
    ├── mod.rs           # TypeChecker struct, pub API (check_core_hir_module など) (~200行)
    ├── builtins.rs      # register_builtins — 全ビルトイン登録 (~400行)
    ├── infer.rs         # 型推論ロジック (infer_type, unify など) (~500行)
    ├── check_expr.rs    # 式の型チェック (literals, calls, binop, field access) (~600行)
    ├── check_fn.rs      # 関数・クロージャ定義のチェック (~400行)
    └── check_stmt.rs    # let/assign/return/block 文のチェック (~400行)
```

## 受け入れ条件

1. 上記 6 ファイルに分割
2. `cargo build --workspace --exclude ark-llvm --exclude ark-lsp` が通る
3. `cargo test --workspace --exclude ark-llvm --exclude ark-lsp` が通る
4. `scripts/verify-harness.sh` が status 0

## 参照

- `crates/ark-typecheck/src/checker.rs`
- `crates/ark-typecheck/src/build_corehir.rs` (別途 1118行 — 今後の課題)
