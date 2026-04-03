# std::collections: BTreeMap、BTreeSet、IndexMap、IndexSet、BitSet

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 046
**Depends on**: 039, 041
**Track**: stdlib
**Blocks v3 exit**: yes


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/046-std-collections-ordered.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

順序付き・挿入順保存・ビット集合のデータ構造を実装する。
BTreeMap/BTreeSet は range query と deterministic order、
IndexMap/IndexSet は挿入順保存 (JSON/stable emit)、
BitSet は密なフラグ集合 (graph/compiler/DFA) に使用。

## 受け入れ条件

### BTreeMap\<K,V\> / BTreeSet\<T\>

```ark
pub fn btree_new<K, V>() -> BTreeMap<K, V>
pub fn btree_insert<K, V>(m: BTreeMap<K, V>, key: K, value: V) -> Option<V>
pub fn btree_get<K, V>(m: BTreeMap<K, V>, key: K) -> Option<V>
pub fn btree_remove<K, V>(m: BTreeMap<K, V>, key: K) -> Option<V>
pub fn btree_contains_key<K, V>(m: BTreeMap<K, V>, key: K) -> bool
pub fn btree_len<K, V>(m: BTreeMap<K, V>) -> i32
pub fn btree_keys<K, V>(m: BTreeMap<K, V>) -> Vec<K>  // sorted order
pub fn btree_range<K, V>(m: BTreeMap<K, V>, start: K, end: K) -> Vec<(K, V)>

pub fn btree_set_new<T>() -> BTreeSet<T>
pub fn btree_set_insert<T>(s: BTreeSet<T>, value: T) -> bool
pub fn btree_set_contains<T>(s: BTreeSet<T>, value: T) -> bool
```

### IndexMap\<K,V\> / IndexSet\<T\>

```ark
pub fn index_map_new<K, V>() -> IndexMap<K, V>
pub fn index_map_insert<K, V>(m: IndexMap<K, V>, key: K, value: V) -> Option<V>
pub fn index_map_get<K, V>(m: IndexMap<K, V>, key: K) -> Option<V>
pub fn index_map_keys<K, V>(m: IndexMap<K, V>) -> Vec<K>  // insertion order
pub fn index_map_entries<K, V>(m: IndexMap<K, V>) -> Vec<(K, V)>  // insertion order

pub fn index_set_new<T>() -> IndexSet<T>
pub fn index_set_insert<T>(s: IndexSet<T>, value: T) -> bool
```

### BitSet

```ark
pub fn bitset_new() -> BitSet
pub fn bitset_with_capacity(cap: i32) -> BitSet
pub fn bitset_set(bs: BitSet, index: i32)
pub fn bitset_clear(bs: BitSet, index: i32)
pub fn bitset_test(bs: BitSet, index: i32) -> bool
pub fn bitset_count(bs: BitSet) -> i32
pub fn bitset_union(a: BitSet, b: BitSet) -> BitSet
pub fn bitset_intersection(a: BitSet, b: BitSet) -> BitSet
```

## 実装タスク

1. `ark-typecheck`: BTreeMap, BTreeSet, IndexMap, IndexSet, BitSet 型の登録
2. BTreeMap: B-tree (order 16) を GC struct ノードで実装
3. IndexMap: HashMap + Vec で insertion order を保持
4. BitSet: GC array of i32, ビット操作で実装
5. `std/collections/btree_map.ark`, `btree_set.ark`, `index_map.ark`, `index_set.ark`, `bit_set.ark`

## 検証方法

- fixture: `stdlib_collections/btree_basic.ark`, `stdlib_collections/btree_range.ark`,
  `stdlib_collections/index_map_order.ark`, `stdlib_collections/bitset_ops.ark`,
  `stdlib_collections/btree_set.ark`, `stdlib_collections/index_set.ark`

## 完了条件

- BTreeMap の keys() が sorted order で返る
- IndexMap の keys() が insertion order で返る
- BitSet の union/intersection が正しい
- fixture 6 件以上 pass

## 注意点

1. BTree 実装の複雑さ: ノード分割・マージが正しく動作することの検証が重い
2. IndexMap の remove 後に insertion order を維持する方法 (tombstone or compact)
3. BitSet の capacity 自動拡張

## ドキュメント

- `docs/stdlib/collections-reference.md` に各構造のセクション追加

## 未解決論点

1. BTree の order (16 vs 32) の最適値
2. IndexMap の remove 時に "swap remove" (O(1) だが order 変化) vs "shift remove" (O(n) だが order 保持)
