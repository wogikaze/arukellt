# Selfhost MIR lowering: 式のコンパイルを実装する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 313
**Depends on**: 308
**Track**: selfhost-backend
**Blocks v1 exit**: no
**Priority**: 3

## Summary

`lower_to_mir()` の NOP stub を実際の式コンパイラに置き換える。これが selfhost backend の最大のボトルネック。現在は全関数が NOP 命令 1 個だけの空ブロックを生成しており、コード生成が一切行われていない。算術演算、比較、変数アクセス、関数呼び出し、リテラルの lowering を実装する。

## Current state

- `src/compiler/mir.ark` (323 行): 43 opcodes 定義済み、MirInst / MirBlock / MirFunction / MirModule の構造体あり
- `lower_to_mir()` は約 13 行のスタブ: typed_fns をループし、各関数に entry block + MIR_NOP 1 個を置くだけ
- Rust 版 `crates/ark-mir/src/lower/expr.rs` は 35,000 行の式コンパイラ
- selfhost MIR lowering のカバー率は 0.5% 未満

## Acceptance

- [x] 算術式 (`1 + 2 * 3`) が正しい MIR 命令列 (CONST_I32, MIR_MUL, MIR_ADD) に lowering される
- [x] `local.get` / `local.set` が変数アクセスに対して生成される
- [x] 関数呼び出しが `MIR_CALL` 命令に lowering される
- [x] string literal が data section に配置される MIR 命令を生成する
- [x] `--dump-phases mir` で生成 MIR を確認できる

## References

- `src/compiler/mir.ark` — selfhost MIR 定義 + lower_to_mir() stub
- `crates/ark-mir/src/lower/expr.rs` — Rust 式コンパイラ (35K 行)
- `crates/ark-mir/src/lower/func.rs` — Rust 関数 CFG 構築 (36K 行)
- `crates/ark-mir/src/mir.rs` — canonical MIR node 定義
