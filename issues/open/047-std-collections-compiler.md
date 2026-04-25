# std::collections: Arena、SlotMap、Interner ／ std::text: Rope

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-22
**ID**: 047
**Depends on**: 039, 041
**Track**: stdlib
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #529
**Blocks v3 exit**: no (Experimental)

**Status note**: Blocker-free stdlib lane. This issue does not carry the #312 generic monomorphization blocker from #044.

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/047-std-collections-compiler.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

コンパイラ・Wasm ツールチェーン・IDE 支援向けの高度なデータ構造を実装する。
Arena (安定 ID と一括所有)、SlotMap (削除に強い handle map)、
Interner (値 ↔ ID 双方向化) を `std::collections` 配下に、
Rope (大きいテキストの編集) を **`std::text::rope`** 配下に実装し、
セルフホストと Wasm tooling の基盤を整備する。

## Operational lane — 2026-04-22

This issue is in the **blocker-free stdlib lane**. Do not hold it behind #312
unless a new STOP_IF is found in this issue's own implementation work.

## 安定性ラベル

この issue の成果物はすべて **Experimental** とする。
API は v4 以降で変更される可能性がある。

## 受け入れ条件

### Arena\<T\>

```ark
pub fn arena_new<T>() -> Arena<T>
pub fn arena_alloc<T>(a: Arena<T>, value: T) -> ArenaId
pub fn arena_get<T>(a: Arena<T>, id: ArenaId) -> Option<T>
pub fn arena_len<T>(a: Arena<T>) -> i32
```

### SlotMap\<V\>

```ark
pub fn slot_map_new<V>() -> SlotMap<V>
pub fn slot_insert<V>(m: SlotMap<V>, value: V) -> SlotKey
pub fn slot_get<V>(m: SlotMap<V>, key: SlotKey) -> Option<V>
pub fn slot_remove<V>(m: SlotMap<V>, key: SlotKey) -> Option<V>
pub fn slot_contains<V>(m: SlotMap<V>, key: SlotKey) -> bool
pub fn slot_len<V>(m: SlotMap<V>) -> i32
```

### Interner\<T\>

```ark
pub fn interner_new<T>() -> Interner<T>
pub fn intern<T>(i: Interner<T>, value: T) -> Symbol
pub fn resolve<T>(i: Interner<T>, sym: Symbol) -> Option<T>
pub fn interner_len<T>(i: Interner<T>) -> i32
```

### Rope (std::text::rope)

`std::text` モジュールの一部として提供する (`std::collections` ではない)。

```ark
pub fn rope_new() -> Rope
pub fn rope_from_string(s: String) -> Rope
pub fn rope_insert(r: Rope, pos: i32, text: String) -> Rope
pub fn rope_delete(r: Rope, start: i32, end: i32) -> Rope
pub fn rope_slice(r: Rope, start: i32, end: i32) -> String
pub fn rope_len(r: Rope) -> i32
pub fn rope_to_string(r: Rope) -> String
pub fn rope_line_count(r: Rope) -> i32
```

## 実装タスク

1. `ark-typecheck`: Arena, ArenaId, SlotMap, SlotKey, Interner, Symbol, Rope 型の登録
2. Arena: GC Vec + 単調増加 ID
3. SlotMap: generation-indexed array (slot = {value, generation})
4. Interner: HashMap<T, Symbol> + Vec<T> の双方向マップ
5. Rope: balanced binary tree of text chunks
6. `std/collections/arena.ark`, `slot_map.ark`, `interner.ark`
7. `std/text/rope.ark` (**`std::text::rope`** namespace に配置)

## 検証方法

- fixture: `stdlib_collections/arena_basic.ark`, `stdlib_collections/slot_map_basic.ark`,
  `stdlib_collections/slot_map_remove.ark`, `stdlib_collections/interner_basic.ark`,
  `stdlib_collections/rope_basic.ark`, `stdlib_collections/rope_edit.ark`

## 完了条件

- Arena の alloc/get が安定 ID で動作する
- SlotMap の remove 後に generation mismatch で None を返す
- Interner が intern/resolve の双方向変換を正しく行う
- Rope の insert/delete が O(log n) で動作する (大きなテキストで検証)
- fixture 6 件以上 pass

## 注意点

1. SlotMap の generation overflow: u32 wrap 時の扱い (panic vs error)
2. Rope のバランス維持: 深さが偏らないよう rebalance を実装
3. これらは Experimental — API 変更の自由度を残す

## ドキュメント

- `docs/stdlib/collections-advanced.md`: Arena, SlotMap, Interner のリファレンス
- `docs/stdlib/modules/text.md` に Rope セクションを追加 (std::text::rope)

## 未解決論点

1. Arena に free/reuse 機能を入れるか (入れると SlotMap との境界が曖昧になる)
2. Rope の chunk size (512 bytes vs 1024 bytes)
3. Interner の型制約 (T に equality + hash が必要だが、現状の型システムで制約できるか)
