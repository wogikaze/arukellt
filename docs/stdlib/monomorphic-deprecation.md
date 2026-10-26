# Deprecated API Migration Table

> Generated from `std/manifest.toml` by `scripts/gen/generate-docs.py`.
> Lifecycle state and replacement are never maintained separately here.
> A replacement string is not proof of current callability; migration evidence remains required.

Deprecated APIs remain callable for the policy window in
[stability-policy.md](stability-policy.md). Monomorphic compatibility
helpers are included alongside any other deprecated public entry.

| API | Module | Stability | Replacement | Deprecated since | Earliest removal | Reason |
|-----|--------|-----------|-------------|------------------|------------------|--------|
| `char_at` | `prelude` | `deprecated` | `byte_at` | `0.1.0` | `1.0.0` | Return the raw byte at a byte index. Use byte_at for the explicit compatibility API. |
| `concat` | `prelude` | `deprecated` | `std::text::concat` | `0.1.0` | `1.0.0` | Concatenate two strings and return the result. |
| `print` | `prelude` | `deprecated` | `std::host::stdio::print` | `0.1.0` | `1.0.0` | Superseded API |
| `println` | `prelude` | `deprecated` | `std::host::stdio::println` | `0.1.0` | `1.0.0` | Superseded API |
| `deque_back` | `std::collections::linear` | `deprecated` | `Deque::back` | `0.1.0` | `1.0.0` | Peek at the back element without removing it. Returns 0 when empty. |
| `deque_clear` | `std::collections::linear` | `deprecated` | `Deque::clear` | `0.1.0` | `1.0.0` | Reset the deque to empty (preserves allocated capacity). |
| `deque_front` | `std::collections::linear` | `deprecated` | `Deque::front` | `0.1.0` | `1.0.0` | Peek at the front element without removing it. Returns 0 when empty. |
| `deque_is_empty` | `std::collections::linear` | `deprecated` | `Deque::is_empty` | `0.1.0` | `1.0.0` | Return true if the deque contains no elements. |
| `deque_len` | `std::collections::linear` | `deprecated` | `Deque::len` | `0.1.0` | `1.0.0` | Return the number of elements in the deque. |
| `deque_new` | `std::collections::linear` | `deprecated` | `Deque::new` | `0.1.0` | `1.0.0` | Create a new empty Deque backed by a ring buffer (i32, monomorphic). |
| `deque_pop_back` | `std::collections::linear` | `deprecated` | `Deque::pop_back` | `0.1.0` | `1.0.0` | Remove and return the back element. Panics if empty. |
| `deque_pop_front` | `std::collections::linear` | `deprecated` | `Deque::pop_front` | `0.1.0` | `1.0.0` | Remove and return the front element. Panics if empty. |
| `deque_push_back` | `std::collections::linear` | `deprecated` | `Deque::push_back` | `0.1.0` | `1.0.0` | Append a value to the back of the deque. |
| `deque_push_front` | `std::collections::linear` | `deprecated` | `Deque::push_front` | `0.1.0` | `1.0.0` | Prepend a value to the front of the deque. |
| `deque_to_vec` | `std::collections::linear` | `deprecated` | `Deque::iter` | `0.1.0` | `1.0.0` | Copy all deque elements (front to back) into a new Vec<i32>. |
| `linked_list_back` | `std::collections::linked_list` | `deprecated` | `LinkedList::back` | `0.1.0` | `1.0.0` | Peek the back value of the legacy i32 list, or the caller's fallback when empty. |
| `linked_list_clear` | `std::collections::linked_list` | `deprecated` | `LinkedList::clear` | `0.1.0` | `1.0.0` | Remove all elements from the legacy i32 list, keeping node capacity. |
| `linked_list_front` | `std::collections::linked_list` | `deprecated` | `LinkedList::front` | `0.1.0` | `1.0.0` | Peek the front value of the legacy i32 list, or the caller's fallback when empty. |
| `linked_list_is_empty` | `std::collections::linked_list` | `deprecated` | `LinkedList::is_empty` | `0.1.0` | `1.0.0` | Return true when the legacy i32 list is empty. |
| `linked_list_len` | `std::collections::linked_list` | `deprecated` | `LinkedList::len` | `0.1.0` | `1.0.0` | Return the number of elements in the legacy i32 list. |
| `linked_list_new` | `std::collections::linked_list` | `deprecated` | `LinkedList::new` | `0.1.0` | `1.0.0` | Create a new empty monomorphic i32 list over a Vec<i32> node pool. |
| `linked_list_pop_back` | `std::collections::linked_list` | `deprecated` | `LinkedList::pop_back` | `0.1.0` | `1.0.0` | Pop the back value of the legacy i32 list, or the caller's fallback when empty. |
| `linked_list_pop_front` | `std::collections::linked_list` | `deprecated` | `LinkedList::pop_front` | `0.1.0` | `1.0.0` | Pop the front value of the legacy i32 list, or the caller's fallback when empty. |
| `linked_list_push_back` | `std::collections::linked_list` | `deprecated` | `LinkedList::push_back` | `0.1.0` | `1.0.0` | Push a value to the back of the legacy i32 list. |
| `linked_list_push_front` | `std::collections::linked_list` | `deprecated` | `LinkedList::push_front` | `0.1.0` | `1.0.0` | Push a value to the front of the legacy i32 list. |
| `linked_list_to_vec` | `std::collections::linked_list` | `deprecated` | `LinkedList::iter` | `0.1.0` | `1.0.0` | Copy the legacy i32 list into a Vec<i32> in front-to-back order. |
| `get_var` | `std::env` | `deprecated` | `var` | `0.1.0` | `1.0.0` | Alias for env::var. Use var instead. |
| `exists` | `std::host::fs` | `deprecated` | `is_readable_file` | `0.1.0` | `1.0.0` | Deprecated alias for is_readable_file. Same read-probe semantics — NOT a general path-existence query. |
| `binary_search` | `std::seq` | `deprecated` | `std::seq::binary_search_by` | `0.1.0` | `1.0.0` | Deprecated monomorphic wrapper. Use the generic std::seq::binary_search_by(v, target) over T: Ord. |
| `count_eq` | `std::seq` | `deprecated` | `std::core::iter::iter_count_eq_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the IteratorCountEq capability v.iter().count_eq(target) or std::core::iter::iter_count_eq_i32(v, target). |
| `filter_i32` | `std::seq` | `deprecated` | `std::core::iter::collect_filter_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().filter(f).collect() or std::core::iter::collect_filter_i32(v, f). |
| `fold_i32_i32` | `std::seq` | `deprecated` | `std::core::iter::iter_fold_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the IteratorFold capability v.iter().fold(init, f) or std::core::iter::iter_fold_i32(v, init, f). |
| `map_i32_i32` | `std::seq` | `deprecated` | `std::core::iter::collect_map_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().map(f).collect() or std::core::iter::collect_map_i32(v, f). |
| `max_i32` | `std::seq` | `deprecated` | `std::seq::max_by` | `0.1.0` | `1.0.0` | Deprecated monomorphic wrapper. Use the generic std::seq::max_by(v) over T: Ord. |
| `min_i32` | `std::seq` | `deprecated` | `std::seq::min_by` | `0.1.0` | `1.0.0` | Deprecated monomorphic wrapper. Use the generic std::seq::min_by(v) over T: Ord. |
| `seq_contains` | `std::seq` | `deprecated` | `std::core::iter::iter_contains_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the IteratorContains capability v.iter().contains(target) or std::core::iter::iter_contains_i32(v, target). |
| `seq_reverse` | `std::seq` | `deprecated` | `std::core::iter::iter_reverse_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().rev().collect() or std::core::iter::iter_reverse_i32(v). |
| `skip_i32` | `std::seq` | `deprecated` | `std::core::iter::collect_skip_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().skip(n).collect() or std::core::iter::collect_skip_i32(v, n). |
| `sum_i32` | `std::seq` | `deprecated` | `std::core::iter::iter_sum_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the IteratorSum capability v.iter().sum() or std::core::iter::iter_sum_i32(v). |
| `take_i32` | `std::seq` | `deprecated` | `std::core::iter::collect_take_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().take(n).collect() or std::core::iter::collect_take_i32(v, n). |
| `unique` | `std::seq` | `deprecated` | `std::core::iter::iter_unique_i32` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().unique().collect() or std::core::iter::iter_unique_i32(v). |

Total deprecated public entries: **41**.
