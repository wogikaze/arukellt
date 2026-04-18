# std::collections::hash: HashMap\<K,V\> 汎用化と HashSet\<T\>

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 044
**Depends on**: 039, 041
**Track**: stdlib
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #39, #41
**Blocks v3 exit**: yes

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: done` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/044-std-collections-hash.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Blocked — 2026-04-04

**Blocked by**: compiler generic monomorphization (#312)

True generic `HashMap<K,V>` and `HashSet<T>` require the compiler to be able to
monomorphize generic type parameters at call sites.  
The impl-stdlib agent hit this blocker (STOP_IF) during Wave 2 dispatch.  
**Do not re-dispatch until #312 (selfhost generic monomorphization) is done.**

## Summary

現在 `HashMap<i32, i32>` のみ存在する HashMap を、任意の K/V 型で使える
汎用 HashMap\<K,V\> に拡張する。また HashSet\<T\> を新設する。
std.md §7.3 で定義された API surface を実装する。

## 背景

現在の HashMap は `HashMap_i32_i32_new()` 等のモノモーフ関数として実装されており、
`HashMap<String, i32>` や `HashMap<String, String>` が使えない。
v3 ではジェネリクスを活用して汎用化する。hash 関数は #041 の std::core::hash に依存。

## 受け入れ条件

### HashMap\<K,V\>

```ark
pub fn new<K, V>() -> HashMap<K, V>
pub fn with_capacity<K, V>(cap: i32) -> HashMap<K, V>
pub fn insert<K, V>(m: HashMap<K, V>, key: K, value: V) -> Option<V>
pub fn get<K, V>(m: HashMap<K, V>, key: K) -> Option<V>
pub fn contains_key<K, V>(m: HashMap<K, V>, key: K) -> bool
pub fn remove<K, V>(m: HashMap<K, V>, key: K) -> Option<V>
pub fn len<K, V>(m: HashMap<K, V>) -> i32
pub fn is_empty<K, V>(m: HashMap<K, V>) -> bool
pub fn clear<K, V>(m: HashMap<K, V>)
pub fn keys<K, V>(m: HashMap<K, V>) -> Vec<K>
pub fn values<K, V>(m: HashMap<K, V>) -> Vec<V>
pub fn entries<K, V>(m: HashMap<K, V>) -> Vec<(K, V)>
```

### HashSet\<T\>

```ark
pub fn set_new<T>() -> HashSet<T>
pub fn set_insert<T>(s: HashSet<T>, value: T) -> bool
pub fn set_contains<T>(s: HashSet<T>, value: T) -> bool
pub fn set_remove<T>(s: HashSet<T>, value: T) -> bool
pub fn set_len<T>(s: HashSet<T>) -> i32
pub fn set_union<T>(a: HashSet<T>, b: HashSet<T>) -> HashSet<T>
pub fn set_intersection<T>(a: HashSet<T>, b: HashSet<T>) -> HashSet<T>
pub fn set_difference<T>(a: HashSet<T>, b: HashSet<T>) -> HashSet<T>
pub fn set_to_vec<T>(s: HashSet<T>) -> Vec<T>
```

## 実装タスク

1. `ark-typecheck`: HashMap/HashSet のジェネリクス登録を一般化
2. `ark-wasm/src/emit`: HashMap の GC 表現 — linear probing, load factor 0.75
3. `std/collections/hash_map.ark`: HashMap 操作関数 (intrinsic + source)
4. `std/collections/hash_set.ark`: HashSet (内部的に HashMap<T, ()> で実装)
5. ハッシュ関数の型別ディスパッチ: i32/i64/String/bool/char のハッシュ計算
6. 旧 `HashMap_i32_i32_*` 関数を deprecated 化

## 検証方法

- fixture: `stdlib_hashmap/hashmap_string_i32.ark`, `stdlib_hashmap/hashmap_string_string.ark`,
  `stdlib_hashmap/hashmap_collision.ark`, `stdlib_hashmap/hashmap_resize.ark`,
  `stdlib_hashmap/hashset_basic.ark`, `stdlib_hashmap/hashset_ops.ark`,
  `stdlib_hashmap/hashmap_generic.ark`
- 既存 `stdlib_hashmap/hashmap_basic.ark` が引き続き pass

## 完了条件

- `HashMap<String, i32>`, `HashMap<String, String>`, `HashMap<i32, String>` が動作する
- HashSet の集合演算 (union/intersection/difference) が正しい
- fixture 7 件以上 pass

## 注意点

1. hash collision 対策: linear probing でバケットが満杯時のリサイズを忘れない
2. GC 表現: HashMap の内部配列は GC array — resize 時に新 array にコピー
3. equality 比較: ジェネリック K に対する equality は型ごとにモノモーフ化で解決

## 次版への受け渡し

- HashMap は std::json (055), std::wit (054) 等で多用される
- IndexMap (#046) は HashMap のバリエーションとして後続実装

## ドキュメント

- `docs/stdlib/collections-reference.md` に HashMap/HashSet セクション追加

## 未解決論点

1. HashMap の key に struct/enum を使えるようにするか (hash/eq の自動導出が必要)
2. `Entry` API (get_or_insert 相当) を v3 に入れるか
3. HashSet の iterate 順序の保証 (なし — 明示する)
