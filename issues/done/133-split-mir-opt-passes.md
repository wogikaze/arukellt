# MIR opt/pipeline.rs (916行) を passes/ ディレクトリに分割

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 133
**Depends on**: —
**Track**: code-structure
**Blocks v4 exit**: no

## Summary

`crates/ark-mir/src/opt/pipeline.rs` は 916 行で、11本の最適化パスの実装が
全て単一ファイルに詰め込まれている。パスごとにファイルを分割し、
新規パス追加時のコンフリクトを減らす。

## 現在の構造

`pipeline.rs` に以下がある:
- `OptimizationPass` enum + `DEFAULT_PASS_ORDER` 定数
- `optimize_module*` エントリポイント関数群
- `run_single_pass` / `run_pass` ディスパッチ
- 11パスの実装 (const_fold, branch_fold, cfg_simplify, copy_prop, const_prop,
  dead_local_elim, dead_block_elim, unreachable_cleanup, inline_small_leaf,
  string_concat_opt, aggregate_simplify)

## 提案する分割後の構造

```
crates/ark-mir/src/opt/
├── pipeline.rs         # 削除
├── mod.rs              # re-export: pub use passes::*, pub use orchestrate::*
├── orchestrate.rs      # optimize_module*, run_single_pass, OptimizationPass enum (~150行)
└── passes/
    ├── mod.rs          # pub use all passes
    ├── const_fold.rs
    ├── branch_fold.rs
    ├── cfg_simplify.rs
    ├── copy_prop.rs
    ├── const_prop.rs
    ├── dead_local_elim.rs
    ├── dead_block_elim.rs
    ├── unreachable_cleanup.rs
    ├── inline_small_leaf.rs
    ├── string_concat_opt.rs
    └── aggregate_simplify.rs
```

各パスファイルは以下の形式:

```rust
// passes/const_fold.rs
use crate::mir::MirFunction;
use super::super::orchestrate::OptimizationSummary;

pub fn const_fold(func: &mut MirFunction) -> OptimizationSummary { ... }
```

## 受け入れ条件

1. 11パスそれぞれが独立したファイルに分離
2. `opt/mod.rs` は従来の `opt/pipeline.rs` と同じ公開 API を維持
3. v4 計画中の LICM, escape_analysis, gc_hint パスを追加する際に
   `passes/` 以下に新ファイルを1つ追加するだけで完結すること
4. `cargo build --workspace --exclude ark-llvm --exclude ark-lsp` が通る
5. `scripts/run/verify-harness.sh` が status 0

## 参照

- `crates/ark-mir/src/opt/pipeline.rs`
- Issue #096–#105 (v4 最適化関連 issues) — 新パスはここに追加される
