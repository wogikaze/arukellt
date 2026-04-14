# 実行時性能: 静的文字列インターニング (data segment 参照)

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-15
**ID**: 106
**Depends on**: —
**Track**: runtime-perf
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/106-runtime-string-interning.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

現在の文字列リテラルは `array.new_data` で毎回新しい GC 配列を生成している。
コンパイル時定数文字列については、プログラム起動時に一度だけ GC 配列を生成し
グローバル変数にキャッシュする「静的インターニング」を導入する。
文字列比較が多いプログラムで GC プレッシャーを大幅削減できる。

## 受け入れ条件

1. コンパイル時定数文字列ごとに `global (ref null $string)` を生成
2. 初回アクセス時に `array.new_data` で初期化し、2回目以降はグローバルを読む
3. `string_concat.ark` ベンチマークで GC 割り当て回数の削減を確認
4. `hello.wasm` バイナリサイズへの影響を確認 (グローバルの追加分)

## 参照

- roadmap-v4.md §2 (hello.wasm 1KB 目標)
