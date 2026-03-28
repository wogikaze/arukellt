# std::collections — コレクション

> **状態**: Vec / HashMap は部分実装済み。HashSet / Deque / BTreeMap / IndexMap / BitSet は v3 で追加予定。

---

## Vec<T>

GC-native な可変長配列。Wasm GC `(array mut ref)` で実装。

### 現行 API (v2 互換)

```ark
// 生成 (v2 モノモーフ名 — v3 で deprecated 予定)
Vec_new_i32()     -> Vec<i32>
Vec_new_i64()     -> Vec<i64>
Vec_new_f64()     -> Vec<f64>
Vec_new_String()  -> Vec<String>

// 基本操作
push(v, x)           // 末尾追加 (in-place)
pop(v)               // 末尾除去
get(v, i) -> T       // 境界チェックあり
get_unchecked(v, i)  // 境界チェックなし
set(v, i, x)         // in-place 書き込み
len(v) -> i32

// ソート
sort_i32(v)  sort_i64(v)  sort_f64(v)  sort_String(v)

// HOF (モノモーフ名 — v3 で deprecated 予定)
map_i32_i32(v, f)    filter_i32(v, f)    fold_i32_i32(v, init, f)
map_i64_i64(v, f)    filter_i64(v, f)    fold_i64_i64(v, init, f)
map_f64_f64(v, f)    filter_f64(v, f)
map_String_String(v, f)  filter_String(v, f)
any_i32(v, f)   find_i32(v, f)

// ユーティリティ
contains_i32(v, x)    contains_String(v, x)
reverse_i32(v)        reverse_String(v)
remove_i32(v, i)
sum_i32(v)            product_i32(v)
```

### v3 新 API (予定)

```ark
// 汎用生成 (型推論でモノモーフ解決)
vec::new<T>() -> Vec<T>
vec::with_capacity<T>(cap: i32) -> Vec<T>
vec::from_seq<T>(s: Seq<T>) -> Vec<T>

// 汎用操作
vec::push<T>(v: Vec<T>, x: T)
vec::pop<T>(v: Vec<T>) -> Option<T>
vec::get<T>(v: Vec<T>, i: i32) -> Option<T>
vec::get_unchecked<T>(v: Vec<T>, i: i32) -> T
vec::set<T>(v: Vec<T>, i: i32, x: T)
vec::len<T>(v: Vec<T>) -> i32
vec::is_empty<T>(v: Vec<T>) -> bool

// 探索・変換
vec::contains<T>(v: Vec<T>, x: T) -> bool
vec::index_of<T>(v: Vec<T>, x: T) -> Option<i32>
vec::reverse<T>(v: Vec<T>)         // in-place
vec::sort<T>(v: Vec<T>)            // Ord required
vec::sort_by<T>(v: Vec<T>, cmp: fn(T, T) -> i32)

// Seq への変換
vec::into_seq<T>(v: Vec<T>) -> Seq<T>
vec::map<T, U>(v: Vec<T>, f: fn(T) -> U) -> Vec<U>
vec::filter<T>(v: Vec<T>, f: fn(T) -> bool) -> Vec<T>
vec::fold<T, U>(v: Vec<T>, init: U, f: fn(U, T) -> U) -> U
vec::any<T>(v: Vec<T>, f: fn(T) -> bool) -> bool
vec::all<T>(v: Vec<T>, f: fn(T) -> bool) -> bool
vec::find<T>(v: Vec<T>, f: fn(T) -> bool) -> Option<T>

// 算術ショートカット
vec::sum_i32(v: Vec<i32>) -> i32
vec::product_i32(v: Vec<i32>) -> i32
```

---

## HashMap<K, V>

オープンアドレッシング (linear probing)、負荷係数 0.75、初期容量 16。

### 現行 API (v2 互換)

```ark
// 生成
HashMap_new_String_i32()    -> HashMap<String, i32>
HashMap_new_i32_i32()       -> HashMap<i32, i32>
HashMap_new_String_String() -> HashMap<String, String>

// 操作
hashmap_insert(m, k, v)              // upsert
hashmap_get(m, k) -> Option<V>
hashmap_contains_key(m, k) -> bool
hashmap_remove(m, k)
hashmap_len(m) -> i32

// イテレーション
hashmap_keys(m) -> Vec<K>
hashmap_values(m) -> Vec<V>
hashmap_entries(m) -> Vec<(K, V)>   // v3
```

### v3 追加 API (予定)

```ark
hashmap_get_or_insert(m, k, default) -> V  // insert if absent, return existing/new
hashmap_update(m, k, f: fn(V) -> V)        // update in-place
hashmap_merge(a, b) -> HashMap<K, V>
hashmap_retain(m, f: fn(K, V) -> bool)     // filter in-place
hashmap_to_seq(m) -> Seq<(K, V)>
hashmap_from_seq(s: Seq<(K, V)>) -> HashMap<K, V>
```

---

## HashSet<T> (v3)

HashMap の key 集合として実装。

```ark
// 生成
hashset_new<T>() -> HashSet<T>
hashset_from_vec<T>(v: Vec<T>) -> HashSet<T>

// 操作
hashset_insert<T>(s: HashSet<T>, x: T) -> bool    // true if newly inserted
hashset_remove<T>(s: HashSet<T>, x: T) -> bool
hashset_contains<T>(s: HashSet<T>, x: T) -> bool
hashset_len<T>(s: HashSet<T>) -> i32
hashset_is_empty<T>(s: HashSet<T>) -> bool

// 集合演算
hashset_union<T>(a: HashSet<T>, b: HashSet<T>) -> HashSet<T>
hashset_intersection<T>(a: HashSet<T>, b: HashSet<T>) -> HashSet<T>
hashset_difference<T>(a: HashSet<T>, b: HashSet<T>) -> HashSet<T>
hashset_is_subset<T>(a: HashSet<T>, b: HashSet<T>) -> bool

// 変換
hashset_to_vec<T>(s: HashSet<T>) -> Vec<T>
hashset_to_seq<T>(s: HashSet<T>) -> Seq<T>
```

---

## Deque<T> (v3)

両端キュー。BFS/DFS などのアルゴリズムで多用。

```ark
deque_new<T>() -> Deque<T>
deque_push_front<T>(d: Deque<T>, x: T)
deque_push_back<T>(d: Deque<T>, x: T)
deque_pop_front<T>(d: Deque<T>) -> Option<T>
deque_pop_back<T>(d: Deque<T>) -> Option<T>
deque_front<T>(d: Deque<T>) -> Option<T>   // peek
deque_back<T>(d: Deque<T>) -> Option<T>    // peek
deque_len<T>(d: Deque<T>) -> i32
deque_is_empty<T>(d: Deque<T>) -> bool
```

---

## BTreeMap<K, V> (v3/v4)

ソート済みキー順に列挙可能な辞書。コンパイラの定数表・シンボルテーブルで有用。

> v3 で設計確定、実装は v4 評価後。

```ark
btree_new<K, V>() -> BTreeMap<K, V>
btree_insert<K, V>(m: BTreeMap<K, V>, k: K, v: V)
btree_get<K, V>(m: BTreeMap<K, V>, k: K) -> Option<V>
btree_remove<K, V>(m: BTreeMap<K, V>, k: K)
btree_len<K, V>(m: BTreeMap<K, V>) -> i32
btree_keys_sorted<K, V>(m: BTreeMap<K, V>) -> Vec<K>  // ascending order
btree_range<K, V>(m: BTreeMap<K, V>, lo: K, hi: K) -> Seq<(K, V)>
```

---

## v3 実装ロードマップ

| コレクション | v3 実装 | issue |
|-------------|---------|-------|
| Vec (汎用 API) | ✅ v3 | [#038](../../issues/open/038-wit-type-fixtures.md) |
| HashMap (完全) | ✅ v3 | [#041](../../issues/open/041-hashmap-complete.md) |
| HashSet | ✅ v3 | [#042](../../issues/open/042-hashset.md) |
| Deque | ✅ v3 | [#041](../../issues/open/041-hashmap-complete.md) (同梱) |
| BTreeMap | 🔮 v4 | ADR-009 評価後 |
| IndexMap | 🔮 v4 | 挿入順保証の HashMap |
| BitSet | 🔮 v4 | 固定サイズビット集合 |
