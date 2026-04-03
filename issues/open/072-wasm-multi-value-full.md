# Wasm Multi-Value: ブロック / ループの複数値返却フル活用

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 072
**Depends on**: —
**Track**: wasm-feature
**Blocks v4 exit**: no


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/072-wasm-multi-value-full.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

WebAssembly Multi-Value 提案 (`docs/spec/spec-1.0.0/proposals/multi-value/Overview.md`) の
「ブロック・ループ・関数が複数の値を返せる」機能を MIR → T3 パイプラインで完全活用する。
現在、複数戻り値を返す関数のうち、ブロック式の途中で複数値を "返す" ケースを
スタックローカルを使って迂回しているパターンがないかを調査・修正する。

## 受け入れ条件

1. 複数戻り値関数の T3 emit が correct であることを確認・テスト追加
2. `if` ブロックの両分岐が同じ多値型を返す場合、ローカル変数を使わず直接スタックに積む
3. `loop` の break 値についても multi-value を活用
4. バイナリサイズ改善 (ローカル変数 set/get の削減) を `wc -c` で確認

## 参照

- `docs/spec/spec-1.0.0/proposals/multi-value/Overview.md`
