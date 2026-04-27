---
Status: done
Created: 2026-03-28
Updated: 2026-04-14
ID: 102
Track: compile-speed
Depends on: 100
Orchestration class: implementation-ready
---
# コンパイル速度: Lexer / Parser のホットパス最適化
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/102-compile-lexer-optimization.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`--time` フラグで計測した後に、lex/parse フェーズが律速している場合のための最適化 issue。
大規模ファイル (500行以上) での lexer のバイト処理速度を改善する。

## 対象最適化

- 文字列スキャンの SIMD 化 (memchr crate による改行/クォート検索)
- トークン種別のテーブルルックアップ最適化 (ASCII 範囲の高速分岐)
- Parser のバックトラック削減 (LL(1) 変換の徹底)

## 受け入れ条件

1. `--time` で lex + parse が `hello.ark` で 5ms 以内であることを確認
2. `parser.ark` (500行) で lex + parse が 50ms 以内
3. `memchr` crate の活用で文字列スキャンを高速化

## 参照

- roadmap-v4.md §2 (コンパイル時間目標)