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
| `assert_eq` | `prelude` | `deprecated` | `Test::assert_eq_i32` | `0.1.0` | `1.0.0` | Superseded API |
| `assert_eq_i64` | `prelude` | `deprecated` | `Test::assert_eq_i64` | `0.1.0` | `1.0.0` | Superseded API |
| `assert_eq_str` | `prelude` | `deprecated` | `Test::assert_eq_string` | `0.1.0` | `1.0.0` | Superseded API |
| `assert_ne` | `prelude` | `deprecated` | `Test::assert_ne_i32` | `0.1.0` | `1.0.0` | Superseded API |
| `get` | `prelude` | `deprecated` | `Vec::get` | `0.1.0` | `1.0.0` | Superseded API |
| `get_unchecked` | `prelude` | `deprecated` | `Vec::get_unchecked` | `0.1.0` | `1.0.0` | Superseded API |
| `len` | `prelude` | `deprecated` | `Vec::len` | `0.1.0` | `1.0.0` | Return the number of elements in a Vec. |
| `pop` | `prelude` | `deprecated` | `Vec::pop` | `0.1.0` | `1.0.0` | Remove and return the last element of a Vec. Returns None if the Vec is empty. |
| `push` | `prelude` | `deprecated` | `Vec::push` | `0.1.0` | `1.0.0` | Append an element to the end of a Vec, mutating it in place. |
| `set` | `prelude` | `deprecated` | `Vec::set` | `0.1.0` | `1.0.0` | Superseded API |
| `slice_get` | `prelude` | `deprecated` | `Slice::get` | `0.1.0` | `1.0.0` | Superseded API |
| `slice_len` | `prelude` | `deprecated` | `Slice::len` | `0.1.0` | `1.0.0` | Superseded API |
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

Total deprecated public entries: **35**.
