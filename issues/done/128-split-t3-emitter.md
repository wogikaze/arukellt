---
Status: done
Created: 2026-03-28
Updated: 2026-04-03
ID: 128
Track: code-structure
Depends on: —
Orchestration class: implementation-ready
---
# T3 GC エミッター (t3_wasm_gc.rs 8678行) をサブモジュールに分割
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: done` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/128-split-t3-emitter.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`crates/ark-wasm/src/emit/t3_wasm_gc.rs` は 8678 行の単一ファイル。
`TypeAlloc` と `Ctx` の実装が全て詰め込まれており、レビュー・差分・IDEの追従が困難。
`emit/t3_wasm_gc/` ディレクトリに分割する。

## 現在のファイル構造

| 行 | 内容 |
|---|---|
| 1–130 | 定数・ヘルパー関数 (mutable_field, ref_nullable, normalize_intrinsic) |
| 131–355 | `impl TypeAlloc` — GC型テーブル管理 |
| 356–562 | `impl Ctx` — Ctx 構造体定義・基本ヘルパー |
| 563–661 | `pub fn emit()` エントリポイント |
| 662–1597 | `impl Ctx::emit_module` — module 構造体 (type sec, import sec, export sec) |
| 1598–3027 | print/format ヘルパー (`emit_print_str_ln_helper`, `emit_i32_to_str_helper` など) |
| 3028–3615 | `emit_function`, `emit_builtin_stub` |
| 3616–3850 | `emit_stmt` |
| 3851–4366 | `emit_operand` |
| 4367–5056 | `emit_call_builtin` — ビルトイン呼び出しディスパッチ |
| 5057–5158 | `emit_try_expr` — 例外処理 |
| 5159–6251 | 文字列操作 (`emit_concat`, `emit_substring_gc`, `emit_string_eq_gc` など) |
| 6252–7148 | 高階関数 (`emit_filter_hof_*`, `emit_map_hof_*`, `emit_fold_hof_*` など) |
| 7149–7519 | ファイルシステム (`emit_fs_read_file_gc`, `emit_fs_write_file_gc`) |
| 7520–8678 | Vec/コレクション操作 + 算術演算 (`emit_vec_new_gc`, `emit_binop`, etc.) |

## 提案する分割後の構造

```
crates/ark-wasm/src/emit/
├── t3_wasm_gc.rs          # 削除 → ディレクトリに変換
└── t3_wasm_gc/
    ├── mod.rs             # エントリポイント (pub fn emit), 定数, 共通ヘルパー (~200行)
    ├── type_alloc.rs      # TypeAlloc impl (~225行)
    ├── ctx.rs             # Ctx 構造体定義 + 基本ヘルパー (~210行)
    ├── module_builder.rs  # emit_module (型セクション・インポート・エクスポート) (~950行)
    ├── print_helpers.rs   # print/format ヘルパー関数群 (~1450行)
    ├── function.rs        # emit_function, emit_builtin_stub (~600行)
    ├── stmt.rs            # emit_stmt (~235行)
    ├── operand.rs         # emit_operand, emit_operand_coerced (~550行)
    ├── builtin.rs         # emit_call_builtin ディスパッチ (~700行)
    ├── exception.rs       # emit_try_expr (~100行)
    ├── string_ops.rs      # 文字列操作 emit_concat_gc, emit_substring_gc など (~1100行)
    ├── hof.rs             # 高階関数 filter/map/fold/any/find (~900行)
    ├── fs.rs              # emit_fs_read_file_gc, emit_fs_write_file_gc (~370行)
    ├── vec.rs             # Vec/コレクション操作 (~800行)
    └── arith.rs           # emit_binop, emit_unaryop (~170行)
```

## 受け入れ条件

1. `t3_wasm_gc.rs` を上記 14 ファイルに分割
2. 各ファイルの `impl Ctx` ブロックは `use super::*;` もしくは `ctx.rs` 経由で `&mut self` アクセス可
   - Rust の impl ブロック分割には `mod` 内で同一型への impl を記述する方法を使う
3. `pub fn emit()` のシグネチャは変更しない
4. `cargo build --workspace --exclude ark-llvm --exclude ark-lsp` が通る
5. `cargo test -p arukellt --test harness` が全テスト green
6. `scripts/run/verify-harness.sh` が status 0

## 注意

- Rust では同一 crate 内なら複数ファイルに `impl Ctx` を分散させられる (ファイルを `mod` として公開し、`use super::Ctx;` する)
- `Ctx` は `pub(crate)` に留める

## 参照

- `crates/ark-wasm/src/emit/t3_wasm_gc.rs`
- `docs/current-state.md` (T3 backend: "fully operational, 415 fixtures pass")