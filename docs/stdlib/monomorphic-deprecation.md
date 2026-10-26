# Deprecated API Migration Table

> Generated from `std/manifest.toml` by `scripts/gen/generate-docs.py`.
> Lifecycle state and replacement are never maintained separately here.
> A replacement string is not proof of current callability; migration evidence remains required.

Deprecated APIs remain callable for the policy window in
[stability-policy.md](stability-policy.md). Monomorphic compatibility
helpers are included alongside any other deprecated public entry.

| API | Module | Stability | Replacement | Deprecated since | Earliest removal | Reason |
|-----|--------|-----------|-------------|------------------|------------------|--------|
| `any_String` | `prelude` | `deprecated` | `VecIter::any` | `0.1.0` | `1.0.0` | Superseded API |
| `any_f64` | `prelude` | `deprecated` | `VecIter::any` | `0.1.0` | `1.0.0` | Superseded API |
| `any_i32` | `prelude` | `deprecated` | `VecIter::any` | `0.1.0` | `1.0.0` | Superseded API |
| `any_i64` | `prelude` | `deprecated` | `VecIter::any` | `0.1.0` | `1.0.0` | Superseded API |
| `as_slice` | `prelude` | `deprecated` | `Vec::as_slice` | `0.1.0` | `1.0.0` | Superseded API |
| `assert_eq` | `prelude` | `deprecated` | `std::test::assert_eq_i32` | `0.1.0` | `1.0.0` | Superseded API |
| `assert_eq_i64` | `prelude` | `deprecated` | `std::test::assert_eq_i64` | `0.1.0` | `1.0.0` | Superseded API |
| `assert_eq_str` | `prelude` | `deprecated` | `std::test::assert_eq_string` | `0.1.0` | `1.0.0` | Superseded API |
| `assert_ne` | `prelude` | `deprecated` | `std::test::assert_ne_i32` | `0.1.0` | `1.0.0` | Superseded API |
| `contains_String` | `prelude` | `deprecated` | `VecIter::contains` | `0.1.0` | `1.0.0` | Superseded API |
| `contains_i32` | `prelude` | `deprecated` | `VecIter::contains` | `0.1.0` | `1.0.0` | Superseded API |
| `contains_string` | `prelude` | `deprecated` | `String::contains` | `0.1.0` | `1.0.0` | Superseded API |
| `f32_to_f64` | `prelude` | `deprecated` | `f32::to_f64` | `0.1.0` | `1.0.0` | Superseded API |
| `f64_bits_hi` | `prelude` | `deprecated` | `f64::to_bits_hi` | `0.1.0` | `1.0.0` | Superseded API |
| `f64_bits_lo` | `prelude` | `deprecated` | `f64::to_bits_lo` | `0.1.0` | `1.0.0` | Superseded API |
| `f64_to_f32` | `prelude` | `deprecated` | `f64::to_f32` | `0.1.0` | `1.0.0` | Superseded API |
| `filter_String` | `prelude` | `deprecated` | `VecIter::filter` | `0.1.0` | `1.0.0` | Superseded API |
| `filter_f64` | `prelude` | `deprecated` | `VecIter::filter` | `0.1.0` | `1.0.0` | Superseded API |
| `filter_i64` | `prelude` | `deprecated` | `VecIter::filter` | `0.1.0` | `1.0.0` | Superseded API |
| `find_String` | `prelude` | `deprecated` | `VecIter::find` | `0.1.0` | `1.0.0` | Superseded API |
| `find_f64` | `prelude` | `deprecated` | `VecIter::find` | `0.1.0` | `1.0.0` | Superseded API |
| `find_i32` | `prelude` | `deprecated` | `VecIter::find` | `0.1.0` | `1.0.0` | Superseded API |
| `find_i64` | `prelude` | `deprecated` | `VecIter::find` | `0.1.0` | `1.0.0` | Superseded API |
| `fold_f64_f64` | `prelude` | `deprecated` | `VecIter::fold` | `0.1.0` | `1.0.0` | Superseded API |
| `fold_i32_i32` | `prelude` | `deprecated` | `VecIter::fold` | `0.1.0` | `1.0.0` | Superseded API |
| `fold_i64_i64` | `prelude` | `deprecated` | `VecIter::fold` | `0.1.0` | `1.0.0` | Superseded API |
| `get` | `prelude` | `deprecated` | `Vec::get` | `0.1.0` | `1.0.0` | Superseded API |
| `get_unchecked` | `prelude` | `deprecated` | `Vec::get_unchecked` | `0.1.0` | `1.0.0` | Superseded API |
| `i16_to_i32` | `prelude` | `deprecated` | `i16::to_i32` | `0.1.0` | `1.0.0` | Superseded API |
| `i32_to_i16` | `prelude` | `deprecated` | `i32::to_i16_wrapping` | `0.1.0` | `1.0.0` | Superseded API |
| `i32_to_i64` | `prelude` | `deprecated` | `i32::into` | `0.1.0` | `1.0.0` | Superseded API |
| `i32_to_i8` | `prelude` | `deprecated` | `i32::to_i8_wrapping` | `0.1.0` | `1.0.0` | Superseded API |
| `i32_to_u16` | `prelude` | `deprecated` | `i32::to_u16_wrapping` | `0.1.0` | `1.0.0` | Superseded API |
| `i32_to_u32` | `prelude` | `deprecated` | `i32::to_u32_bits` | `0.1.0` | `1.0.0` | Superseded API |
| `i32_to_u8` | `prelude` | `deprecated` | `i32::to_u8_wrapping` | `0.1.0` | `1.0.0` | Superseded API |
| `i64_to_i32` | `prelude` | `deprecated` | `i64::to_i32_checked` | `0.1.0` | `1.0.0` | Superseded API |
| `len` | `prelude` | `deprecated` | `Vec::len` | `0.1.0` | `1.0.0` | Return the number of elements in a Vec. |
| `map_String_String` | `prelude` | `deprecated` | `VecIter::map` | `0.1.0` | `1.0.0` | Superseded API |
| `map_f64_f64` | `prelude` | `deprecated` | `VecIter::map` | `0.1.0` | `1.0.0` | Superseded API |
| `map_i32_i32` | `prelude` | `deprecated` | `VecIter::map` | `0.1.0` | `1.0.0` | Superseded API |
| `map_i64_i64` | `prelude` | `deprecated` | `VecIter::map` | `0.1.0` | `1.0.0` | Superseded API |
| `map_option_i32_i32` | `prelude` | `deprecated` | `Option::map` | `0.1.0` | `1.0.0` | Superseded API |
| `pop` | `prelude` | `deprecated` | `Vec::pop` | `0.1.0` | `1.0.0` | Remove and return the last element of a Vec. Returns None if the Vec is empty. |
| `product_i32` | `prelude` | `deprecated` | `VecIter::fold` | `0.1.0` | `1.0.0` | Superseded API |
| `push` | `prelude` | `deprecated` | `Vec::push` | `0.1.0` | `1.0.0` | Append an element to the end of a Vec, mutating it in place. |
| `remove_i32` | `prelude` | `deprecated` | `Vec::drain` | `0.1.0` | `1.0.0` | Superseded API |
| `reverse_String` | `prelude` | `deprecated` | `VecIter::rev` | `0.1.0` | `1.0.0` | Superseded API |
| `reverse_i32` | `prelude` | `deprecated` | `VecIter::rev` | `0.1.0` | `1.0.0` | Superseded API |
| `set` | `prelude` | `deprecated` | `Vec::set` | `0.1.0` | `1.0.0` | Superseded API |
| `slice_get` | `prelude` | `deprecated` | `Slice::get` | `0.1.0` | `1.0.0` | Superseded API |
| `slice_len` | `prelude` | `deprecated` | `Slice::len` | `0.1.0` | `1.0.0` | Superseded API |
| `sort_String` | `prelude` | `deprecated` | `Vec::sort_by` | `0.1.0` | `1.0.0` | Superseded API |
| `sort_f64` | `prelude` | `deprecated` | `Vec::sort_by` | `0.1.0` | `1.0.0` | Superseded API |
| `sort_i32` | `prelude` | `deprecated` | `Vec::sort_by` | `0.1.0` | `1.0.0` | Superseded API |
| `sort_i64` | `prelude` | `deprecated` | `Vec::sort_by` | `0.1.0` | `1.0.0` | Superseded API |
| `sum_i32` | `prelude` | `deprecated` | `VecIter::sum` | `0.1.0` | `1.0.0` | Superseded API |
| `u16_to_i32` | `prelude` | `deprecated` | `u16::to_i32` | `0.1.0` | `1.0.0` | Superseded API |
| `u32_to_u64` | `prelude` | `deprecated` | `u32::to_u64` | `0.1.0` | `1.0.0` | Superseded API |
| `u64_to_u32` | `prelude` | `deprecated` | `u64::to_u32_wrapping` | `0.1.0` | `1.0.0` | Superseded API |
| `u8_to_i32` | `prelude` | `deprecated` | `u8::to_i32` | `0.1.0` | `1.0.0` | Superseded API |
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

Total deprecated public entries: **97**.
