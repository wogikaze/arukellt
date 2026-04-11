# T1 エミッター (t1_wasm32_p1.rs 9465行) をサブモジュールに分割

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 129
**Depends on**: —
**Track**: code-structure
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/129-split-t1-emitter.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`crates/ark-wasm/src/emit/t1_wasm32_p1.rs` は 9465 行の単一ファイル。
全処理が単一 `impl EmitCtx` ブロックに詰め込まれており、
コード探索・差分レビュー・機能追加が困難。
`emit/t1_wasm32_p1/` ディレクトリに分割する。

## 現在のファイル構造

| 行 | 内容 |
|---|---|
| 1–82 | インポート・定数 |
| 83–153 | `fn normalize_intrinsic_name` |
| 154–248 | `pub fn emit()` エントリポイント + EmitCtx 構造体定義 |
| 249–424 | `impl EmitCtx` 基本ヘルパー |
| 425–3261 | `emit_module` — 型セクション・インポート・エクスポート・メモリ・データ |
| 3262–3523 | `emit_bump_alloc` — バンプアロケータ |
| 3524–3760 | `emit_stmt` — ステートメントディスパッチ |
| 3761–4023 | `emit_println`, `emit_eprintln`, `emit_print`, `emit_fd_write` |
| 4024–9137 | `emit_operand` — オペランド (巨大: ~5100行) |
| 9138–9256 | `emit_binop`, `emit_binop_i64`, `emit_binop_f64` |
| 9257–9465 | `emit_unaryop`, `emit_f64_operand`, `emit_i64_operand` |

## 提案する分割後の構造

```
crates/ark-wasm/src/emit/
├── t1_wasm32_p1.rs          # 削除 → ディレクトリに変換
└── t1_wasm32_p1/
    ├── mod.rs               # エントリポイント (pub fn emit), 定数, EmitCtx struct (~250行)
    ├── module_builder.rs    # emit_module: 型/インポート/エクスポート/メモリ (~2850行)
    ├── alloc.rs             # emit_bump_alloc (~265行)
    ├── stmt.rs              # emit_stmt (~240行)
    ├── io.rs                # emit_println, emit_eprintln, emit_print, emit_fd_write (~265行)
    ├── operand.rs           # emit_operand (~5100行) ※さらに分割可能
    └── arith.rs             # emit_binop*, emit_unaryop, emit_*_operand (~330行)
```

### `operand.rs` のさらなる分割（任意）

`emit_operand` (~5100行) の内部はビルトイン呼び出しのディスパッチが大半。
フェーズ2として以下に分割可能：

```
    ├── operand/
    │   ├── mod.rs           # emit_operand ディスパッチ
    │   ├── builtin_str.rs   # 文字列ビルトイン
    │   ├── builtin_vec.rs   # Vec ビルトイン
    │   ├── builtin_math.rs  # 数学関数
    │   └── builtin_io.rs    # I/O ビルトイン
```

## 受け入れ条件

1. `t1_wasm32_p1.rs` を上記ファイルに分割
2. `pub fn emit()` のシグネチャは変更しない
3. `cargo build --workspace --exclude ark-llvm --exclude ark-lsp` が通る
4. `cargo test -p arukellt --test harness` が全テスト green
5. `scripts/run/verify-harness.sh` が status 0

## 参照

- `crates/ark-wasm/src/emit/t1_wasm32_p1.rs`
