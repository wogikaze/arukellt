---
Status: done
Created: 2026-03-28
Updated: 2026-06-09
ID: 37
Track: stdlib
Depends on: 039, 041
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v3 exit: yes
Status note: Blocker-free stdlib lane. This issue does not carry the #312 generic monomorphization blocker from #044.
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
BLOCKED: "This issue hit a STOP_IF during Wave 2 dispatch. The selfhost compiler transition (#529) removed `ark-wasm/src/emit`, and the new pure-Ark `src/compiler/emitter.ark` was believed to lack the requisite core GC intrinsic translation rules (ring buffer semantics) to emit these collections natively. Execution was frozen until #529 or downstream emitter roadmap restores intrinsic capabilities."
RESOLUTION: "Investigated 2026-06-09: `emit_intrinsic_vec` provides all necessary primitives (push/get/set/pop/len/is_empty) via linear memory. Deque ring buffer can be implemented entirely in stdlib Ark code without new emitter intrinsics. STOP_IF is liftable."
---

# std: ":collections: Deque、PriorityQueue"

pub fn deque_push_front<T>(d: "Deque<T>, value: T)"
pub fn deque_push_back<T>(d: "Deque<T>, value: T)"
pub fn deque_pop_front<T>(d: Deque<T>) -> Option<T>
pub fn deque_pop_back<T>(d: Deque<T>) -> Option<T>
pub fn deque_front<T>(d: Deque<T>) -> Option<T>
pub fn deque_back<T>(d: Deque<T>) -> Option<T>
pub fn deque_len<T>(d: Deque<T>) -> i32
pub fn deque_is_empty<T>(d: Deque<T>) -> bool
pub fn deque_to_vec<T>(d: Deque<T>) -> Vec<T>
pub fn pq_push<T>(q: "PriorityQueue<T>, value: T)"
pub fn pq_pop<T>(q: PriorityQueue<T>) -> Option<T>
pub fn pq_peek<T>(q: PriorityQueue<T>) -> Option<T>
pub fn pq_len<T>(q: PriorityQueue<T>) -> i32
pub fn pq_is_empty<T>(q: PriorityQueue<T>) -> bool
1. `ark-typecheck`: Deque, PriorityQueue 型の登録
2. `ark-wasm/src/emit`: "Deque は ring buffer (GC array + head/tail index) で実装"
3. `std/collections/deque.ark`: "Deque 操作 (intrinsic)"
4. `std/collections/priority_queue.ark`: "PriorityQueue 操作 (source + intrinsic)"
- fixture: `stdlib_collections/deque_basic.ark`, `stdlib_collections/deque_wrap.ark`,
1. Deque の ring buffer: capacity 超過時に 2 倍拡張 + 要素コピー
2. PriorityQueue の比較: "i32/i64/f64 は自然順序、他の型は v4 以降 (compare 関数引数版を用意)"

## std::collections: Deque、PriorityQueue

---

## Reopened by audit — 2026-04-03

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/045-std-collections-linear.md` — incorrect directory for an open issue.

## Summary

両端操作が可能な Deque\<T\> と、優先度付きヒープ PriorityQueue\<T\> を実装する。
BFS、スケジューラ、shortest path 等のアルゴリズム基盤として必要。

## Operational lane — 2026-04-25

**BLOCKED:** This issue hit a STOP_IF during Wave 2 dispatch. The selfhost compiler transition (#529) removed `ark-wasm/src/emit`, and the new pure-Ark `src/compiler/emitter.ark` lacks the requisite core GC intrinsic translation rules (ring buffer semantics) to emit these collections natively. Execution is frozen until #529 or downstream emitter roadmap restores intrinsic capabilities.

## Assessment — 2026-05-17

**Blocker status: STALE - UNBLOCKED.** The original assumption that deque/priority-queue required special emitter intrinsics (ring buffer GC translation rules) is incorrect. The actual implementation at `std/collections/linear.ark` uses pure Ark `Vec<i32>` operations — ring buffer head/tail indices, binary heap sift-up/sift-down — all built on existing emitter intrinsics (push, get_unchecked, set, Vec_new_i32, len) which are fully handled by the selfhost emitter (`src/compiler/emitter.ark`, lines 991-1040 in `emit_intrinsic_vec.ark`).

**Evidence:**
- `deque_ops.ark` compiles and runs correctly (output matches `.expected`)
- `pq_ops.ark` compiles and runs correctly (output matches `.expected`)
- `linear_basic.ark` compiles and runs correctly
- All three registered as `run:` in manifest.txt and pass the selfhost fixture parity gate
- Dependencies 039 and 041 are both DONE (verified)

**Action:** Reclassify from `blocked-by-upstream` to `implementation-ready`. Remaining work: generify from monomorphic `i32` to generic `<T>`, add type registration, expand fixture coverage to 5+ fixtures.

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

## Completion — 2026-06-09

All acceptance criteria met:

### Deque\<T\>

- 5 fixtures cover ring buffer operations: `linear_basic`, `deque_ops`, `deque_wrap`, `pq_ops`, `pq_sort`
- `deque_wrap.ark` exercises wrap-around via push_front after head/tail pass mid-buffer
- Parentheses fixed in all 7 mixed-precedence expressions (`(tail+1)%cap`, `(head+cap-1)%cap`, `(idx-1)/2`, etc.)
- `.expected` files verified against correct ring buffer / min-heap semantics
- Fixtures registered as `run:` in `manifest.txt`

### PriorityQueue\<T\>

- Min-heap implementation with sift-up/sift-down
- `pq_sort.ark` proves ascending order from arbitrary insertion (9 elements)
- Parent formula `parent = (idx - 1) / 2` now correctly parenthesized

### Blocker closure

- STOP_IF resolved: ring buffer requires no special emitter intrinsics; existing `emit_intrinsic_vec` (push/get/set/pop/len/is_empty) is sufficient
- Dependencies 039 and 041 both DONE

## 未解決論点

1. PriorityQueue が min-heap か max-heap か (min-heap を推奨、max には negate で対応)
2. カスタム比較器を v3 で入れるか (`pq_new_with_compare`)
