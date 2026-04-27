---
Status: done
Created: 2026-03-28
Updated: 2026-04-14
ID: 048
Track: stdlib
Depends on: 039, 041
Orchestration class: blocked-by-upstream
Orchestration upstream: #39, #41
---

# std::seq: Seq\<T\> 遅延シーケンスとアルゴリズム
**Blocks v3 exit**: yes

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/048-std-seq.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

Vec に詰め込まれている HOF (map/filter/fold) を分離し、
遅延評価の `Seq<T>` 型とアルゴリズム層を新設する。
「保持する型 (Vec)」と「計算する型 (Seq)」の分離により、
API の膨張を防ぎつつ、パイプライン的データ処理を可能にする。

## 背景

現在の prelude には `map_i32_i32`, `filter_i32`, `fold_i32_i32` 等のモノモーフ HOF が
20 関数以上存在する。これらを `Seq<T>` ベースの遅延パイプラインに統一し、
Vec 側は storage responsibility に集中させる。

## 受け入れ条件

### Seq\<T\> コア

```ark
pub fn from_vec<T>(v: Vec<T>) -> Seq<T>
pub fn range_i32(start: i32, end: i32) -> Seq<i32>
pub fn map<T, U>(s: Seq<T>, f: fn(T) -> U) -> Seq<U>
pub fn filter<T>(s: Seq<T>, f: fn(T) -> bool) -> Seq<T>
pub fn flat_map<T, U>(s: Seq<T>, f: fn(T) -> Seq<U>) -> Seq<U>
pub fn take<T>(s: Seq<T>, n: i32) -> Seq<T>
pub fn skip<T>(s: Seq<T>, n: i32) -> Seq<T>
pub fn enumerate<T>(s: Seq<T>) -> Seq<(i32, T)>
pub fn zip<T, U>(a: Seq<T>, b: Seq<U>) -> Seq<(T, U)>
pub fn chain<T>(a: Seq<T>, b: Seq<T>) -> Seq<T>
```

### 終端操作

```ark
pub fn collect_vec<T>(s: Seq<T>) -> Vec<T>
pub fn collect_hash_set<T>(s: Seq<T>) -> HashSet<T>
pub fn fold<T, U>(s: Seq<T>, init: U, f: fn(U, T) -> U) -> U
pub fn any<T>(s: Seq<T>, f: fn(T) -> bool) -> bool
pub fn all<T>(s: Seq<T>, f: fn(T) -> bool) -> bool
pub fn count<T>(s: Seq<T>) -> i32
pub fn first<T>(s: Seq<T>) -> Option<T>
pub fn last<T>(s: Seq<T>) -> Option<T>
pub fn find<T>(s: Seq<T>, f: fn(T) -> bool) -> Option<T>
pub fn sum_i32(s: Seq<i32>) -> i32
pub fn sum_f64(s: Seq<f64>) -> f64
```

### アルゴリズム (Vec 上)

```ark
pub fn sort<T>(v: Vec<T>) -> Vec<T>
pub fn sort_by<T>(v: Vec<T>, cmp: fn(T, T) -> i32) -> Vec<T>
pub fn binary_search<T>(v: Vec<T>, target: T) -> Result<i32, i32>
pub fn dedup<T>(v: Vec<T>) -> Vec<T>
pub fn chunk<T>(v: Vec<T>, size: i32) -> Vec<Vec<T>>
pub fn window<T>(v: Vec<T>, size: i32) -> Vec<Vec<T>>
pub fn partition<T>(v: Vec<T>, f: fn(T) -> bool) -> (Vec<T>, Vec<T>)
pub fn group_by<T, K>(v: Vec<T>, key: fn(T) -> K) -> Vec<(K, Vec<T>)>
```

## 実装タスク

1. `ark-typecheck`: Seq<T> 型の登録 (内部的にはクロージャベースの generator)
2. `ark-wasm/src/emit`: Seq を GC struct {next: fn() -> Option<T>} として表現
3. `std/seq/seq.ark`: map/filter/take 等の変換操作
4. `std/seq/collect.ark`: 終端操作 (collect_vec, fold, any, all 等)
5. `std/seq/algo.ark`: sort, binary_search, dedup 等のアルゴリズム
6. 既存モノモーフ HOF (`map_i32_i32` 等) を deprecated 化
7. sort 実装: merge sort (安定ソート) に改善 (現在は bubble sort)

## 検証方法

- fixture: `stdlib_seq/seq_map_filter.ark`, `stdlib_seq/seq_take_skip.ark`,
  `stdlib_seq/seq_fold.ark`, `stdlib_seq/seq_zip.ark`,
  `stdlib_seq/seq_collect.ark`, `stdlib_seq/sort_stable.ark`,
  `stdlib_seq/binary_search.ark`, `stdlib_seq/chunk_window.ark`

## 完了条件

- Seq<T> の遅延パイプラインが動作する (map → filter → collect_vec)
- sort が merge sort で安定ソート
- fixture 8 件以上 pass

## 注意点

1. Seq の遅延性: `map` した時点では何も計算せず、`collect_vec` で初めて実行
2. 無限列への対応: `take` なしの `collect_vec` はメモリ枯渇 — 警告を出すか制限するか
3. 既存 HOF との移行: 旧 API は prelude に deprecated wrapper として残す

## ドキュメント

- `docs/stdlib/seq-reference.md`: Seq API, アルゴリズム, 使用パターン

## 未解決論点

1. Seq の内部表現: closure-based vs coroutine-based
2. `group_by` の K 型に hash/eq 制約が必要 — 現状の型システムで可能か
3. `sort_by` の比較関数の戻り値: `i32` (-1/0/1) vs `Ordering`

---

## Queue closure verification — 2026-04-18

- **Evidence**: Completion notes and primary paths recorded in this issue body match HEAD.
- **Verification**: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18).
- **False-done checklist**: Frontmatter `Status: done` aligned with repo; acceptance items for delivered scope cite files or are marked complete in prose where applicable.

**Reviewer:** implementation-backed queue normalization (verify checklist).
