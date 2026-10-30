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

Total deprecated public entries: **41**.
