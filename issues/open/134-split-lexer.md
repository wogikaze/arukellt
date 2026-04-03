# ark-lexer lib.rs (1698行) をサブモジュールに分割

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 134
**Depends on**: —
**Track**: code-structure
**Blocks v4 exit**: no


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/134-split-lexer.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`crates/ark-lexer/src/lib.rs` は 1698 行。
`Token` enum の定義・キーワードテーブル・スキャンロジックが1ファイルに混在。
`lib.rs` を残しつつサブモジュールに分割する。

## 提案する分割後の構造

```
crates/ark-lexer/src/
├── lib.rs          # Lexer struct + pub fn lex() エントリポイント (~200行)
├── token.rs        # Token enum + TokenKind (~300行)
├── keywords.rs     # キーワードテーブル (str → Token マッピング) (~100行)
└── scan.rs         # スキャン実装: scan_ident, scan_number, scan_string, scan_comment (~1100行)
```

## 受け入れ条件

1. 上記 4 ファイルに分割
2. `cargo build --workspace --exclude ark-llvm --exclude ark-lsp` が通る
3. `cargo test --workspace --exclude ark-llvm --exclude ark-lsp` が通る
4. `scripts/run/verify-harness.sh` が status 0

## 参照

- `crates/ark-lexer/src/lib.rs`
