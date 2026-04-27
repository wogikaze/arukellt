---
Status: done
Created: 2026-03-28
Updated: 2026-04-03
ID: 111
Track: benchmark
Depends on: —
Orchestration class: implementation-ready
---
# ツール: Wasm バイナリサイズ内訳分析ツール
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/111-bench-wasm-size-analysis.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

生成した `.wasm` ファイルのサイズ内訳を
セクション別・関数別・型定義別に可視化するツールを追加する。
`wasm-objdump` の出力をパースして Arukellt ソースの関数名とマッピングする。

## 受け入れ条件

1. `arukellt analyze --wasm-size output.wasm` コマンドで内訳表示
2. セクション別 (type, import, function, memory, global, export, data, code) のサイズ表示
3. 上位10関数のコードサイズランキング
4. 出力例:

   ```
   code section: 4.2KB (73%)
     fib:           128B
     print_i64:     512B
     ...
   type section: 0.8KB (14%)
   data section: 0.3KB (5%)
   ```

## 参照

- roadmap-v4.md §11 (docs/process/benchmark-results.md)