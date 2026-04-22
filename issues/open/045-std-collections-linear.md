# std::collections: Deque、PriorityQueue

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-22
**ID**: 045
**Depends on**: 039, 041
**Track**: stdlib
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v3 exit**: yes

**Status note**: Blocker-free stdlib lane. This issue does not carry the #312 generic monomorphization blocker from #044.

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/045-std-collections-linear.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

両端操作が可能な Deque\<T\> と、優先度付きヒープ PriorityQueue\<T\> を実装する。
BFS、スケジューラ、shortest path 等のアルゴリズム基盤として必要。

## Operational lane — 2026-04-22

This issue is in the **blocker-free stdlib lane**. Do not hold it behind #312
unless a new STOP_IF is found in this issue's own implementation work.

## 受け入れ条件

### Deque\<T\>

```ark
pub fn deque_new<T>() -> Deque<T>
pub fn deque_push_front<T>(d: Deque<T>, value: T)
pub fn deque_push_back<T>(d: Deque<T>, value: T)
pub fn deque_pop_front<T>(d: Deque<T>) -> Option<T>
pub fn deque_pop_back<T>(d: Deque<T>) -> Option<T>
pub fn deque_front<T>(d: Deque<T>) -> Option<T>
pub fn deque_back<T>(d: Deque<T>) -> Option<T>
pub fn deque_len<T>(d: Deque<T>) -> i32
pub fn deque_is_empty<T>(d: Deque<T>) -> bool
pub fn deque_to_vec<T>(d: Deque<T>) -> Vec<T>
```

### PriorityQueue\<T\>

```ark
pub fn pq_new<T>() -> PriorityQueue<T>
pub fn pq_push<T>(q: PriorityQueue<T>, value: T)
pub fn pq_pop<T>(q: PriorityQueue<T>) -> Option<T>
pub fn pq_peek<T>(q: PriorityQueue<T>) -> Option<T>
pub fn pq_len<T>(q: PriorityQueue<T>) -> i32
pub fn pq_is_empty<T>(q: PriorityQueue<T>) -> bool
```

## 実装タスク

1. `ark-typecheck`: Deque, PriorityQueue 型の登録
2. `ark-wasm/src/emit`: Deque は ring buffer (GC array + head/tail index) で実装
3. `std/collections/deque.ark`: Deque 操作 (intrinsic)
4. PriorityQueue は binary heap (GC array + sift_up/sift_down) で実装
5. `std/collections/priority_queue.ark`: PriorityQueue 操作 (source + intrinsic)

## 検証方法

- fixture: `stdlib_collections/deque_basic.ark`, `stdlib_collections/deque_wrap.ark`,
  `stdlib_collections/pq_basic.ark`, `stdlib_collections/pq_sort.ark`,
  `stdlib_collections/deque_to_vec.ark`

## 完了条件

- Deque が ring buffer で正しく動作する (wrap-around 含む)
- PriorityQueue が min-heap として正しく動作する
- fixture 5 件以上 pass

## 注意点

1. Deque の ring buffer: capacity 超過時に 2 倍拡張 + 要素コピー
2. PriorityQueue の比較: i32/i64/f64 は自然順序、他の型は v4 以降 (compare 関数引数版を用意)
3. Deque と Vec の API 名衝突を避ける (deque_push_front vs push)

## ドキュメント

- `docs/stdlib/collections-reference.md` に Deque/PriorityQueue セクション追加

## 未解決論点

1. PriorityQueue が min-heap か max-heap か (min-heap を推奨、max には negate で対応)
2. カスタム比較器を v3 で入れるか (`pq_new_with_compare`)
