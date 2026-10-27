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
| `base64_decode` | `std::bytes` | `deprecated` | `Bytes::from_base64` | `0.1.0` | `1.0.0` | Decode a standard base64 string into a byte buffer. Silently skips invalid characters. |
| `base64_encode` | `std::bytes` | `deprecated` | `Vec::to_base64` | `0.1.0` | `1.0.0` | Encode a byte buffer as standard base64 (RFC 4648, with padding). |
| `buf_extend` | `std::bytes` | `deprecated` | `Vec::extend_bytes` | `0.1.0` | `1.0.0` | Append all bytes from a Bytes slice onto a ByteBuf. |
| `buf_freeze` | `std::bytes` | `deprecated` | `Vec::freeze` | `0.1.0` | `1.0.0` | Freeze a ByteBuf into an immutable Bytes value. In the current runtime both are Vec<i32>, so this is identity. |
| `buf_len` | `std::bytes` | `deprecated` | `Vec::len` | `0.1.0` | `1.0.0` | Return the number of bytes in a ByteBuf. |
| `buf_new` | `std::bytes` | `deprecated` | `Vec::new` | `0.1.0` | `1.0.0` | Create a new empty ByteBuf (represented as Vec<i32>). |
| `buf_push_u16_le` | `std::bytes` | `deprecated` | `Vec::push_u16_le` | `0.1.0` | `1.0.0` | Push a 16-bit value in little-endian byte order onto a ByteBuf. |
| `buf_push_u32_le` | `std::bytes` | `deprecated` | `Vec::push_u32_le` | `0.1.0` | `1.0.0` | Push a 32-bit value in little-endian byte order onto a ByteBuf. |
| `buf_push_u64_le` | `std::bytes` | `deprecated` | `Vec::push_u64_le` | `0.1.0` | `1.0.0` | Push a 64-bit value in little-endian byte order onto a ByteBuf. x is i64. |
| `buf_push_u8` | `std::bytes` | `deprecated` | `Vec::push_byte` | `0.1.0` | `1.0.0` | Push a single byte (0-255) onto a ByteBuf. |
| `buf_with_capacity` | `std::bytes` | `deprecated` | `Vec::with_capacity` | `0.1.0` | `1.0.0` | Create a new ByteBuf with a capacity hint. Capacity is advisory only in the current runtime. |
| `byte_length` | `std::bytes` | `deprecated` | `String::len_bytes` | `0.1.0` | `1.0.0` | Superseded API |
| `bytes_concat` | `std::bytes` | `deprecated` | `Vec::concat_bytes` | `0.1.0` | `1.0.0` | Superseded API |
| `bytes_eq` | `std::bytes` | `deprecated` | `Vec::equals_bytes` | `0.1.0` | `1.0.0` | Superseded API |
| `bytes_from_array` | `std::bytes` | `deprecated` | `Bytes::from_array` | `0.1.0` | `1.0.0` | Create an immutable Bytes value from an array of byte values (Vec<i32>, values 0-255). NOTE: Vec<u8> is not a distinct type; Vec<i32> is used. |
| `bytes_from_string` | `std::bytes` | `deprecated` | `String::to_utf8_bytes` | `0.1.0` | `1.0.0` | Superseded API |
| `bytes_get` | `std::bytes` | `deprecated` | `Vec::get_unchecked` | `0.1.0` | `1.0.0` | Superseded API |
| `bytes_len` | `std::bytes` | `deprecated` | `Vec::len` | `0.1.0` | `1.0.0` | Superseded API |
| `bytes_new` | `std::bytes` | `deprecated` | `Vec::new` | `0.1.0` | `1.0.0` | Superseded API |
| `bytes_push` | `std::bytes` | `deprecated` | `Vec::push_byte` | `0.1.0` | `1.0.0` | Superseded API |
| `bytes_slice` | `std::bytes` | `deprecated` | `Vec::slice_bytes` | `0.1.0` | `1.0.0` | Superseded API |
| `cursor_new` | `std::bytes` | `deprecated` | `Vec::byte_cursor` | `0.1.0` | `1.0.0` | Create a ByteCursor from a Bytes value. Index 0 is the position; remaining indices are data bytes. |
| `cursor_pos` | `std::bytes` | `deprecated` | `Vec::cursor_pos` | `0.1.0` | `1.0.0` | Return the current byte position of a ByteCursor. |
| `cursor_remaining` | `std::bytes` | `deprecated` | `Vec::cursor_remaining` | `0.1.0` | `1.0.0` | Return the number of bytes remaining in a ByteCursor. |
| `hex_decode` | `std::bytes` | `deprecated` | `Bytes::from_hex` | `0.1.0` | `1.0.0` | Superseded API |
| `hex_encode` | `std::bytes` | `deprecated` | `Vec::to_hex` | `0.1.0` | `1.0.0` | Superseded API |
| `leb128_decode_i32` | `std::bytes` | `deprecated` | `Vec::read_leb128_i32` | `0.1.0` | `1.0.0` | Decode a signed LEB128 i32 from a ByteCursor. Advances the cursor. |
| `leb128_decode_u32` | `std::bytes` | `deprecated` | `Vec::read_leb128_u32` | `0.1.0` | `1.0.0` | Decode an unsigned LEB128 u32 from a ByteCursor. Advances the cursor. |
| `leb128_decode_u64` | `std::bytes` | `deprecated` | `Vec::read_leb128_u64` | `0.1.0` | `1.0.0` | Decode an unsigned LEB128 u64 from a ByteCursor. Returns i64. Advances the cursor. |
| `leb128_encode_i32` | `std::bytes` | `deprecated` | `Bytes::from_leb128_i32` | `0.1.0` | `1.0.0` | Superseded API |
| `leb128_encode_u32` | `std::bytes` | `deprecated` | `Bytes::from_leb128_u32` | `0.1.0` | `1.0.0` | Superseded API |
| `leb128_encode_u64` | `std::bytes` | `deprecated` | `Bytes::from_leb128_u64` | `0.1.0` | `1.0.0` | Encode a 64-bit unsigned value as LEB128 bytes. x is passed as i64. NOTE: u64 is not a distinct type. |
| `read_bytes` | `std::bytes` | `deprecated` | `Vec::read_bytes` | `0.1.0` | `1.0.0` | Read n bytes from a ByteCursor into a new Bytes buffer. Returns empty buffer on underflow. |
| `read_u16_le` | `std::bytes` | `deprecated` | `Vec::read_u16_le` | `0.1.0` | `1.0.0` | Read two bytes little-endian from a ByteCursor. Returns -1 on underflow. |
| `read_u32_be` | `std::bytes` | `deprecated` | `Vec::read_u32_be` | `0.1.0` | `1.0.0` | Read four bytes big-endian from a ByteCursor. Returns -1 on underflow. |
| `read_u32_le` | `std::bytes` | `deprecated` | `Vec::read_u32_le` | `0.1.0` | `1.0.0` | Read four bytes little-endian from a ByteCursor. Returns -1 on underflow. |
| `read_u64_le` | `std::bytes` | `deprecated` | `Vec::read_u64_le` | `0.1.0` | `1.0.0` | Read eight bytes little-endian from a ByteCursor as i64. Returns -1 on underflow. |
| `read_u8` | `std::bytes` | `deprecated` | `Vec::read_u8` | `0.1.0` | `1.0.0` | Read one byte from a ByteCursor and advance the position. Returns -1 on underflow. |
| `string_from_bytes` | `std::bytes` | `deprecated` | `String::from_utf8_lossy` | `0.1.0` | `1.0.0` | Superseded API |
| `u16_from_le_bytes` | `std::bytes` | `deprecated` | `Vec::to_u16_le` | `0.1.0` | `1.0.0` | Superseded API |
| `u16_to_le_bytes` | `std::bytes` | `deprecated` | `Bytes::from_u16_le` | `0.1.0` | `1.0.0` | Superseded API |
| `u32_from_be_bytes` | `std::bytes` | `deprecated` | `Vec::to_u32_be` | `0.1.0` | `1.0.0` | Superseded API |
| `u32_from_le_bytes` | `std::bytes` | `deprecated` | `Vec::to_u32_le` | `0.1.0` | `1.0.0` | Superseded API |
| `u32_to_be_bytes` | `std::bytes` | `deprecated` | `Bytes::from_u32_be` | `0.1.0` | `1.0.0` | Superseded API |
| `u32_to_le_bytes` | `std::bytes` | `deprecated` | `Bytes::from_u32_le` | `0.1.0` | `1.0.0` | Superseded API |
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
| `writer_write` | `std::io` | `deprecated` | `Writer::write` | `0.1.0` | `1.0.0` | Deprecated tagged write for stdout() / stderr(); use Writer::write for memory handles. Stays until std::host::stdio exposes typed Write handles (ADR-085 D9). |
| `writer_write_str` | `std::io` | `deprecated` | `Writer::write_string` | `0.1.0` | `1.0.0` | Deprecated tagged string write for stdout() / stderr(); use Writer::write_string for memory handles. Stays until std::host::stdio exposes typed Write handles (ADR-085 D9). |
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

Total deprecated public entries: **113**.
