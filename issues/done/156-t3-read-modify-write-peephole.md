# T3 backend-opt: `struct.get` → 即時 `struct.set` 系の read-modify-write 最適化

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-04-15
**ID**: 156
**Depends on**: —
**Track**: backend-opt
**Blocks v1 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/156-t3-read-modify-write-peephole.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`docs/process/roadmap-v4.md` §5.3 は、T3 backend 最適化として
`struct.get` + 即座 `struct.set` を含む field access/update パターンの削減を求めている。
`local.get/set` peephole・文字列リテラル dedup・定数条件 `if` 除去は既存 issue で追跡済みだが、
この read-modify-write 系パターンだけが open queue に独立 issue として存在していない。

## 受け入れ条件

1. T3 emit で削減したい `struct.get` / `struct.set` パターンを具体化し、適用条件を文書化する
2. emit 中または emit 後 peephole で、意味論を変えずに冗長な field read/update 命令列を削減できる
3. `--opt-level 0` では無効、`--opt-level 1` 以上で有効など opt-level 条件を明示する
4. fixture / baseline で最適化ありなしの意味論的同値性を確認する

## 実装タスク

1. `crates/ark-wasm/src/emit/t3*` の field access/update パターンを棚卸しする
2. `struct.get` → 即時 `struct.set` のうち安全に縮約できる read-modify-write パターンを定義する
3. 既存 peephole / post-emit 最適化との責務分担を整理する
4. `docs/compiler/pipeline.md` または同等の最適化説明に反映できる形で最適化意図を記録する

## 実装 (2026-04-15)

**実装ファイル**:
- `crates/ark-wasm/src/emit/t3_wasm_gc/peephole.rs` — `RmwState` ステートマシンを追加;
  4命令パターン `local.get $x; local.get $x; struct.get $T $F; struct.set $T $F`
  (同一ローカル・同一フィールド) を検出して全4命令を除去する `PeepholeWriter` Pass 2;
  `rmw_count` カウンタと `rmw_count()` アクセサを追加.
- `crates/ark-wasm/src/emit/t3_wasm_gc/helpers.rs` — `tee_count` と同様に `rmw_count` をログ記録.

**テストフィクスチャ**:
- `tests/fixtures/structs/rmw_noop.ark` + `rmw_noop.expected` — `obj.field = obj.field` パターンで
  RMW peephole が発火し、意味論的同値性を確認.
- `tests/fixtures/manifest.txt` に `t3-run:structs/rmw_noop.ark` を追加.

**受け入れ條件の確認**:
1. 消去対象パターンをモジュール doc コメントに文書化 ✓
2. `PeepholeWriter` Pass 2 として emit 中に冗長 field read/update 列を除去 ✓
3. `opt_level >= 1` で有効、`opt_level 0` では無効 (enabled フラグ) ✓
4. `rmw_noop.ark` fixture で最適化あり・なしの意味論的同値性を確認 ✓

## 参照

- `docs/process/roadmap-v4.md` §5.3
- `issues/done/088-t3-peephole-local-getset.md`
- `issues/done/095-t3-struct-layout-opt.md`
- `crates/ark-wasm/src/emit/t3_wasm_gc.rs`
