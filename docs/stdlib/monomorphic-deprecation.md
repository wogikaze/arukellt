# Deprecated API Migration Table

> Generated from `std/manifest.toml` by `scripts/gen/generate-docs.py`.
> Lifecycle state and replacement are never maintained separately here.
> A replacement string is not proof of current callability; migration evidence remains required.

Deprecated APIs remain callable for the policy window in
[stability-policy.md](stability-policy.md). Monomorphic compatibility
helpers are included alongside any other deprecated public entry.

| API | Module | Stability | Replacement | Deprecated since | Earliest removal | Reason |
|-----|--------|-----------|-------------|------------------|------------------|--------|
| `as_slice` | `prelude` | `deprecated` | `Vec::as_slice` | `0.1.0` | `1.0.0` | Superseded API |
| `assert_eq` | `prelude` | `deprecated` | `std::test::assert_eq_i32` | `0.1.0` | `1.0.0` | Superseded API |
| `assert_eq_i64` | `prelude` | `deprecated` | `std::test::assert_eq_i64` | `0.1.0` | `1.0.0` | Superseded API |
| `assert_eq_str` | `prelude` | `deprecated` | `std::test::assert_eq_string` | `0.1.0` | `1.0.0` | Superseded API |
| `assert_ne` | `prelude` | `deprecated` | `std::test::assert_ne_i32` | `0.1.0` | `1.0.0` | Superseded API |
| `f32_to_f64` | `prelude` | `deprecated` | `f32::to_f64` | `0.1.0` | `1.0.0` | Superseded API |
| `f64_bits_hi` | `prelude` | `deprecated` | `f64::to_bits_hi` | `0.1.0` | `1.0.0` | Superseded API |
| `f64_bits_lo` | `prelude` | `deprecated` | `f64::to_bits_lo` | `0.1.0` | `1.0.0` | Superseded API |
| `f64_to_f32` | `prelude` | `deprecated` | `f64::to_f32` | `0.1.0` | `1.0.0` | Superseded API |
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
| `i8_to_i32` | `prelude` | `deprecated` | `i8::to_i32` | `0.1.0` | `1.0.0` | Superseded API |
| `len` | `prelude` | `deprecated` | `Vec::len` | `0.1.0` | `1.0.0` | Return the number of elements in a Vec. |
| `pop` | `prelude` | `deprecated` | `Vec::pop` | `0.1.0` | `1.0.0` | Remove and return the last element of a Vec. Returns None if the Vec is empty. |
| `push` | `prelude` | `deprecated` | `Vec::push` | `0.1.0` | `1.0.0` | Append an element to the end of a Vec, mutating it in place. |
| `set` | `prelude` | `deprecated` | `Vec::set` | `0.1.0` | `1.0.0` | Superseded API |
| `slice_get` | `prelude` | `deprecated` | `Slice::get` | `0.1.0` | `1.0.0` | Superseded API |
| `slice_len` | `prelude` | `deprecated` | `Slice::len` | `0.1.0` | `1.0.0` | Superseded API |
| `u16_to_i32` | `prelude` | `deprecated` | `u16::to_i32` | `0.1.0` | `1.0.0` | Superseded API |
| `u32_to_u64` | `prelude` | `deprecated` | `u32::to_u64` | `0.1.0` | `1.0.0` | Superseded API |
| `u64_to_u32` | `prelude` | `deprecated` | `u64::to_u32_wrapping` | `0.1.0` | `1.0.0` | Superseded API |
| `u8_to_i32` | `prelude` | `deprecated` | `u8::to_i32` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_capacity` | `std::collections::hash` | `deprecated` | `HashMap::capacity` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_clear` | `std::collections::hash` | `deprecated` | `HashMap::clear` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_contains` | `std::collections::hash` | `deprecated` | `HashMap::contains_key` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_drain` | `std::collections::hash` | `deprecated` | `HashMap::drain` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_entries` | `std::collections::hash` | `deprecated` | `HashMap::entries` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_extend` | `std::collections::hash` | `deprecated` | `HashMap::extend` | `0.1.0` | `1.0.0` | Extend a raw i32->i32 HashMap with all key/value pairs from another map. Existing keys are overwritten by the source map. Mirrors Rust's HashMap::extend. |
| `hashmap_get` | `std::collections::hash` | `deprecated` | `HashMap::get` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_get_option` | `std::collections::hash` | `deprecated` | `HashMap::get` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_insert` | `std::collections::hash` | `deprecated` | `HashMap::insert` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_is_empty` | `std::collections::hash` | `deprecated` | `HashMap::is_empty` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_keys` | `std::collections::hash` | `deprecated` | `HashMap::keys` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_new` | `std::collections::hash` | `deprecated` | `HashMap::new` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_remove` | `std::collections::hash` | `deprecated` | `HashMap::remove` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_remove_entry` | `std::collections::hash` | `deprecated` | `HashMap::remove_entry` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_reserve` | `std::collections::hash` | `deprecated` | `HashMap::reserve` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_set` | `std::collections::hash` | `deprecated` | `HashMap::insert` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_shrink_to_fit` | `std::collections::hash` | `deprecated` | `HashMap::shrink_to_fit` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_size` | `std::collections::hash` | `deprecated` | `HashMap::len` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_try_reserve` | `std::collections::hash` | `deprecated` | `HashMap::try_reserve` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_values` | `std::collections::hash` | `deprecated` | `HashMap::values` | `0.1.0` | `1.0.0` | Superseded API |
| `hashmap_with_capacity` | `std::collections::hash` | `deprecated` | `HashMap::with_capacity` | `0.1.0` | `1.0.0` | Superseded API |
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
| `bitset_mark` | `std::collections::ordered` | `deprecated` | `BitSet::mark` | `0.1.0` | `1.0.0` | Superseded API |
| `bitset_new` | `std::collections::ordered` | `deprecated` | `BitSet::new` | `0.1.0` | `1.0.0` | Superseded API |
| `bitset_test` | `std::collections::ordered` | `deprecated` | `BitSet::test` | `0.1.0` | `1.0.0` | Superseded API |
| `bitset_unmark` | `std::collections::ordered` | `deprecated` | `BitSet::unmark` | `0.1.0` | `1.0.0` | Superseded API |
| `sorted_map_contains` | `std::collections::ordered` | `deprecated` | `SortedMap::contains` | `0.1.0` | `1.0.0` | Superseded API |
| `sorted_map_find_idx` | `std::collections::ordered` | `deprecated` | `SortedMap::find_idx` | `0.1.0` | `1.0.0` | Superseded API |
| `sorted_map_get` | `std::collections::ordered` | `deprecated` | `SortedMap::get` | `0.1.0` | `1.0.0` | Superseded API |
| `sorted_map_insert` | `std::collections::ordered` | `deprecated` | `SortedMap::insert` | `0.1.0` | `1.0.0` | Superseded API |
| `sorted_map_len` | `std::collections::ordered` | `deprecated` | `SortedMap::len` | `0.1.0` | `1.0.0` | Superseded API |
| `sorted_map_new` | `std::collections::ordered` | `deprecated` | `SortedMap::new` | `0.1.0` | `1.0.0` | Superseded API |
| `binary_search` | `std::seq` | `deprecated` | `std::seq::binary_search_by` | `0.1.0` | `1.0.0` | Deprecated monomorphic wrapper. Use the generic std::seq::binary_search_by(v, target) over T: Ord. |
| `count_eq` | `std::seq` | `deprecated` | `VecIter::count_eq` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the IteratorCountEq capability v.iter().count_eq(target). |
| `filter_i32` | `std::seq` | `deprecated` | `VecIter::filter` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().filter(f).collect(). |
| `fold_i32_i32` | `std::seq` | `deprecated` | `VecIter::fold` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the IteratorFold capability v.iter().fold(init, f). |
| `map_i32_i32` | `std::seq` | `deprecated` | `VecIter::map` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().map(f).collect(). |
| `max_i32` | `std::seq` | `deprecated` | `std::seq::max_by` | `0.1.0` | `1.0.0` | Deprecated monomorphic wrapper. Use the generic std::seq::max_by(v) over T: Ord. |
| `min_i32` | `std::seq` | `deprecated` | `std::seq::min_by` | `0.1.0` | `1.0.0` | Deprecated monomorphic wrapper. Use the generic std::seq::min_by(v) over T: Ord. |
| `seq_contains` | `std::seq` | `deprecated` | `VecIter::contains` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the IteratorContains capability v.iter().contains(target). |
| `seq_reverse` | `std::seq` | `deprecated` | `VecIter::rev` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().rev().collect(). |
| `skip_i32` | `std::seq` | `deprecated` | `VecIter::skip` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().skip(n).collect(). |
| `sum_i32` | `std::seq` | `deprecated` | `VecIter::sum` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the IteratorSum capability v.iter().sum(). |
| `take_i32` | `std::seq` | `deprecated` | `VecIter::take` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().take(n).collect(). |
| `unique` | `std::seq` | `deprecated` | `VecIter::unique` | `0.1.0` | `1.0.0` | Deprecated eager helper. Use the lazy pipeline v.iter().unique().collect(). |

Total deprecated public entries: **96**.
