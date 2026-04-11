# T3: struct フィールドレイアウト最適化 (アクセス頻度ベース)

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 095
**Depends on**: —
**Track**: backend-opt
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/095-t3-struct-layout-opt.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

Wasm GC の struct 型はフィールドインデックスでアクセスされるため、
フィールドの並び順を変えても意味論は変わらない。
アクセス頻度の高いフィールドを低インデックスに置くことで、
wasmtime の JIT コードがより小さいオフセット即値を使えるようになる。
将来の WasmGC struct packing 最適化への布石にもなる。

## 受け入れ条件

1. MIR の `struct_def` にフィールドアクセス頻度カウントを追加
2. T3 emit 時にアクセス頻度の高いフィールドを前方に並べ替えた型定義を生成
3. フィールドインデックスのリマッピングを `struct.get` / `struct.set` 全箇所に適用
4. `--opt-level 2` でのみ有効

## 参照

- `docs/spec/spec-3.0.0/proposals/gc/MVP.md` §struct
