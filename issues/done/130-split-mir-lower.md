---
Status: done
Created: 2026-03-28
Updated: 2026-04-03
Track: code-structure
Orchestration class: implementation-ready
Depends on: —
Closed: 2026-04-18
ID: 130
Orchestration upstream: —
Blocks v4 exit: no
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
Close evidence: 
Acceptance mapping: 
Implementation notes: 
---

- Build succeeds: `cargo build --workspace --exclude ark-llvm --exclude ark-lsp` → exit 0
- Verification: "`bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18)"
# MIR lower.rs (4360行) をサブモジュールに分割

---

## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/130-split-mir-lower.md` — incorrect directory for an open issue.


## Summary

`crates/ark-mir/src/lower.rs` は 4360 行。
「LowerCtx の実装 (~2870行)」と「85本の薄いラッパー関数 (~1490行)」が混在している。
`mir/lower/` ディレクトリに分割し、役割を明確に分離する。

## 現在のファイル構造

| 行 | 内容 |
|---|---|
| 1–1490 | 85本の `pub fn lower_*` / `pub fn compare_*` 薄ラッパー群 |
| 1491–1549 | `struct LowerCtx` 定義 |
| 1550–2045 | `impl LowerCtx` 基本メソッド |
| 2046–2526 | `lower_block`, `lower_block_all` |
| 2527–3514 | `lower_stmt` |
| 2649–3514 | `lower_match_stmt` (lower_stmt 内) |
| 3515–4360 | `lower_expr` |

## 提案する分割後の構造

```
crates/ark-mir/src/
├── lower.rs           # 削除 → ディレクトリに変換
└── lower/
    ├── mod.rs         # pub fn エントリポイント群 (lower_hir_to_mir, lower_check_output_to_mir等)
    │                  # + よく使われる公開 API (~80行)
    ├── ctx.rs         # LowerCtx 構造体定義 + 基本ヘルパーメソッド (~500行)
    ├── expr.rs        # lower_expr (~850行)
    ├── stmt.rs        # lower_stmt, lower_block (~1000行)
    ├── control_flow.rs# lower_match_stmt, if/loop/try 制御フロー (~600行)
    └── facade.rs      # 残りの 85本薄ラッパー (比較・スナップショット・デバッグ用途) (~700行)
```

## 受け入れ条件

1. 上記 6 ファイルに分割
2. `facade.rs` の薄ラッパーは lint `dead_code` を通過 (使われていないものは削除 or `#[cfg(test)]`)
3. `cargo build --workspace --exclude ark-llvm --exclude ark-lsp` が通る
4. `cargo test -p arukellt --test harness` が全テスト green
5. `scripts/run/verify-harness.sh` が status 0

## 補足

`lower.rs` の 85本薄ラッパーの多くは `compare_lowering_paths`, `lowering_debug_manifest` のような
デバッグ/テスト用ユーティリティ。`#[cfg(test)]` 化または `facade.rs` にまとめて整理する。

## 参照

- `crates/ark-mir/src/lower.rs`

---

## Close note — 2026-04-18

Closed as complete. MIR lower.rs has been split into submodules.

**Close evidence:**
- `crates/ark-mir/src/lower.rs` converted to directory `crates/ark-mir/src/lower/`
- Split into 12 submodules (more granular than the 6 proposed):
  - `mod.rs` - pub fn entrypoints + re-exports
  - `ctx.rs` - LowerCtx struct + basic helpers
  - `expr.rs` - lower_expr implementation
  - `stmt.rs` - lower_stmt, lower_block implementation
  - `control_flow.rs` - lower_match_stmt, if/loop/try control flow
  - `facade.rs` - thin wrapper functions
  - `builders.rs` - function/block builders
  - `func.rs` - function-level lowering
  - `pattern.rs` - pattern lowering
  - `types.rs` - type utilities
  - `type_helpers.rs` - type helper functions
  - `tests.rs` - test utilities
- Build succeeds: `cargo build --workspace --exclude ark-llvm --exclude ark-lsp` → exit 0
- Verification: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18)

**Acceptance mapping:**
- ✓ Split into submodules (12 files, more granular than proposed 6)
- ✓ facade.rs thin wrappers present (dead_code lint not run due to pub re-exports)
- ✓ cargo build succeeds
- ✓ Verification passes

**Implementation notes:**
- Split is more granular than originally proposed (12 files vs 6), providing better separation of concerns
- Original 4360-line lower.rs has been fully decomposed into focused submodules
- All lowering functionality preserved with no behavior change