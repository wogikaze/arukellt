# std::seq — 遅延パイプライン

> **状態**: 未実装。v3 で設計・実装予定。

---

## 設計方針

`Vec<T>` に `map` / `filter` 等を直接生やすと中間 Vec が都度生成され非効率になる場合がある。  
`Seq<T>` は遅延評価のパイプラインを表し、`collect_vec()` で初めて具体化する。

```
vec::into_seq(v)
  |> seq::filter(f)
  |> seq::map(g)
  |> seq::take(10)
  |> seq::collect_vec()   -- ここで1回だけ評価
```

---

## Seq<T> 生成

```ark
pub fn seq_from_vec<T>(v: Vec<T>) -> Seq<T>
pub fn seq_range_i32(start: i32, end: i32) -> Seq<i32>         // [start, end)
pub fn seq_range_inclusive(start: i32, end: i32) -> Seq<i32>   // [start, end]
pub fn seq_repeat<T>(x: T, n: i32) -> Seq<T>
pub fn seq_once<T>(x: T) -> Seq<T>
pub fn seq_empty<T>() -> Seq<T>
```

---

## 変換 (lazy — 中間 Vec を生成しない)

```ark
pub fn seq_map<T, U>(s: Seq<T>, f: fn(T) -> U) -> Seq<U>
pub fn seq_filter<T>(s: Seq<T>, f: fn(T) -> bool) -> Seq<T>
pub fn seq_flat_map<T, U>(s: Seq<T>, f: fn(T) -> Seq<U>) -> Seq<U>
pub fn seq_take<T>(s: Seq<T>, n: i32) -> Seq<T>
pub fn seq_skip<T>(s: Seq<T>, n: i32) -> Seq<T>
pub fn seq_take_while<T>(s: Seq<T>, f: fn(T) -> bool) -> Seq<T>
pub fn seq_skip_while<T>(s: Seq<T>, f: fn(T) -> bool) -> Seq<T>
pub fn seq_enumerate<T>(s: Seq<T>) -> Seq<(i32, T)>
pub fn seq_zip<A, B>(a: Seq<A>, b: Seq<B>) -> Seq<(A, B)>
pub fn seq_chain<T>(a: Seq<T>, b: Seq<T>) -> Seq<T>
pub fn seq_flat<T>(s: Seq<Seq<T>>) -> Seq<T>               // = flat_map(id)
pub fn seq_inspect<T>(s: Seq<T>, f: fn(T)) -> Seq<T>       // デバッグ用 side-effect
```

---

## 収集 (eager — ここで評価)

```ark
pub fn seq_collect_vec<T>(s: Seq<T>) -> Vec<T>
pub fn seq_collect_hashset<T>(s: Seq<T>) -> HashSet<T>
pub fn seq_collect_hashmap<K, V>(s: Seq<(K, V)>) -> HashMap<K, V>
pub fn seq_collect_string(s: Seq<String>) -> String        // join without sep
pub fn seq_collect_string_sep(s: Seq<String>, sep: String) -> String
```

---

## 折り畳み・集約

```ark
pub fn seq_fold<T, U>(s: Seq<T>, init: U, f: fn(U, T) -> U) -> U
pub fn seq_reduce<T>(s: Seq<T>, f: fn(T, T) -> T) -> Option<T>
pub fn seq_count<T>(s: Seq<T>) -> i32
pub fn seq_sum_i32(s: Seq<i32>) -> i32
pub fn seq_sum_i64(s: Seq<i64>) -> i64
pub fn seq_product_i32(s: Seq<i32>) -> i32
pub fn seq_min_i32(s: Seq<i32>) -> Option<i32>
pub fn seq_max_i32(s: Seq<i32>) -> Option<i32>
pub fn seq_any<T>(s: Seq<T>, f: fn(T) -> bool) -> bool
pub fn seq_all<T>(s: Seq<T>, f: fn(T) -> bool) -> bool
pub fn seq_find<T>(s: Seq<T>, f: fn(T) -> bool) -> Option<T>
pub fn seq_position<T>(s: Seq<T>, f: fn(T) -> bool) -> Option<i32>
```

---

## アルゴリズム (Vec 上の eager)

```ark
// ソート
pub fn sort_by<T>(v: Vec<T>, cmp: fn(T, T) -> i32) -> Vec<T>    // 安定ソート
pub fn sort_stable<T>(v: Vec<T>) -> Vec<T>                        // Ord 必要

// 検索
pub fn binary_search_i32(v: Vec<i32>, x: i32) -> Result<i32, i32>
// Ok(index) if found, Err(insert_position) if not

// グループ化
pub fn group_by<K, T>(s: Seq<T>, key: fn(T) -> K) -> HashMap<K, Vec<T>>
pub fn partition<T>(s: Seq<T>, f: fn(T) -> bool) -> (Vec<T>, Vec<T>)

// 重複除去
pub fn dedup_i32(v: Vec<i32>) -> Vec<i32>       // 連続重複除去 (O(n))
pub fn unique_i32(v: Vec<i32>) -> Vec<i32>       // 全重複除去 (O(n log n))
pub fn unique_by<T>(v: Vec<T>, key: fn(T) -> i32) -> Vec<T>

// ウィンドウ・チャンク
pub fn seq_windows<T>(s: Seq<T>, size: i32) -> Seq<Vec<T>>    // sliding window
pub fn seq_chunks<T>(s: Seq<T>, size: i32) -> Seq<Vec<T>>     // non-overlapping
```

---

## 使用例

```ark
use std::seq

// 1-100 の偶数の二乗の合計
let total = seq_range_i32(1, 101)
    |> seq_filter(fn(x) { x % 2 == 0 })
    |> seq_map(fn(x) { x * x })
    |> seq_sum_i32()

// 単語の出現頻度
fn word_count(words: Vec<String>) -> HashMap<String, i32> {
    seq_from_vec(words)
        |> seq_fold(HashMap_new_String_i32(), fn(acc, word) {
            let count = hashmap_get(acc, word)
            match count {
                Some(n) => hashmap_insert(acc, word, n + 1),
                None    => hashmap_insert(acc, word, 1),
            }
            acc
        })
}
```

---

## v3 実装ロードマップ

`Seq<T>` は GC-native な generator/closure 表現として実装。  
各変換は closure を保持する GC struct の連鎖として実装し、`collect_*` 呼び出し時にチェーンを評価する。

実装 issue: [#043](../../issues/open/043-seq-lazy-pipeline.md)
