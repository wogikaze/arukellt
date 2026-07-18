# Research: 10 remaining Memory64 T3 WASM validation failures

Generated: 2026-07-18
Target: `wasm32-gc` with Memory64
Compiler: `.build/selfhost/arukellt-s2-runtime.wasm`

## Tracking
- Issue: #808 (`issues/open/808-t3-wasm-validation-failures.md`)
- Parent: #686 (Wasm GC backend completeness)
- ADR / Plan: ADR-035 / `docs/plans/wasm-gc-implementation.md`
- Current state: `docs/current-state.md` notes `wasm32-gc` GC lowering is partial and
enum/Option/Result still use linear-memory discriminated unions.

## Executive summary

These 10 fixtures produce wasm32-gc + Memory64 modules that fail `wasm-tools validate`.
They cluster into four categories:

1. **i64 vs i32 pointer width at WASI/host imports** (`host_module_contract`)
2. **i64 (linear-memory enum/integer payload) passed where a GC ref is expected**
(`json_perf_decode`, `buf_read`, `ord_sort_by`)
3. **ref nullability mismatch** (`wit_type_basic`, `toml_full_*`)
4. **GC type identity / local type mismatch / stack underflow**
(`hashmap_generic_demo`, `io_copy`, `hash_trait`)

Most failures are not isolated emitter bugs; they reflect incomplete GC layout
migration for enums/Options/Results and missing ABI adaptation between Memory64
i64 addresses and WASI P2 i32 pointer imports.

## stdlib_hashmap/hashmap_generic_demo.ark

- **Function index**: `58` (`hashmap_str_str_get`)
- **Offset**: `0x3425`
- **Function header**: `(func (;58;) (type 69))`

### Validation error
```
error: func 58 failed to validate

Caused by:
    0: type mismatch: expected (ref null $type), found (ref null $type) (at offset 0x3425)
```

### Dump context
```
 0x341b | 01 63 1f    | 1 locals of type Ref((ref null (module 31)))
 0x341e | 01 63 1f    | 1 locals of type Ref((ref null (module 31)))
 0x3421 | 01 6b       | 1 locals of type Ref(structref)
 0x3423 | 20 00       | local_get local_index:0
 0x3425 | 21 02       | local_set local_index:2
 0x3427 | 20 01       | local_get local_index:1
 0x3429 | fb 17 07    | ref_cast_nullable hty:Concrete(Module(7))
 0x342c | fb 17 07    | ref_cast_nullable hty:Concrete(Module(7))
 0x342f | 21 03       | local_set local_index:3
```

### Fixture source (first 50 lines)
```ark
use std::collections::hash
use std::collections::hash_map
use std::test

fn main() {
    // --- HashMap<String, i32> via wrapper API ---
    let m = hash_map::hashmap_str_i32_new()
    let _ = hash_map::hashmap_str_i32_insert(m, "alpha", 1)
    let _ = hash_map::hashmap_str_i32_insert(m, "beta", 2)
    let _ = hash_map::hashmap_str_i32_insert(m, "alpha", 10)
    test::assert_eq_i32(hash_map::hashmap_str_i32_len(m), 2)
    match hash_map::hashmap_str_i32_get(m, "alpha") {
        Option::Some(v) => test::assert_eq_i32(v, 10),
        None => panic("expected Some(10)"),
    }
    match hash_map::hashmap_str_i32_get(m, "gamma") {
        Option::Some(v) => panic("expected None for gamma"),
        None => {
        },
    }
    test::assert_true(hash_map::hashmap_str_i32_contains(m, "beta"))
    test::assert_false(hash_map::hashmap_str_i32_contains(m, "unknown"))

    // --- HashMap<i32, String> via wrapper API ---
    let m2 = hash_map::hashmap_i32_str_new()
    let _ = hash_map::hashmap_i32_str_insert(m2, 1, "one")
    let _ = hash_map::hashmap_i32_str_insert(m2, 2, "two")
    let _ = hash_map::hashmap_i32_str_insert(m2, 1, "ONE")
    test::assert_eq_i32(hash_map::hashmap_i32_str_len(m2), 2)
    match hash_map::hashmap_i32_str_get(m2, 1) {
        Option::Some(v) => test::assert_eq_string(v, "ONE"),
        None => panic("expected Some(\"ONE\")"),
    }

    // --- HashMap<String, String> via wrapper API ---
    let m3 = hash_map::hashmap_str_str_new()
    let _ = hash_map::hashmap_str_str_insert(m3, "greeting", "hello")
    let _ = hash_map::hashmap_str_str_insert(m3, "farewell", "goodbye")
    test::assert_eq_i32(hash_map::hashmap_str_str_len(m3), 2)
    match hash_map::hashmap_str_str_get(m3, "greeting") {
        Option::Some(v) => test::assert_eq_string(v, "hello"),
        None => panic("expected Some(\"hello\")"),
    }
    test::assert_true(hash_map::hashmap_str_str_contains(m3, "farewell"))

    // --- HashSet<String> via wrapper API ---
    let s = hash_map::hashset_str_new()
    test::assert_true(hash_map::hashset_str_insert(s, "red"))
    test::assert_true(hash_map::hashset_str_insert(s, "green"))
    test::assert_false(hash_map::hashset_str_insert(s, "red"))
```

### WASM function start
```wat
(func (;58;) (type 69) (param (ref null 25) (ref null 7)) (result (ref null 21))
    (local (ref null 17) (ref null 7) (ref null 21) i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 f64 f64 f64 i64 (ref ...
    local.get 0
    local.set 2
    local.get 1
    ref.cast (ref null 7)
    ref.cast (ref null 7)
    local.set 3
    local.get 2
    local.get 3
    local.set 5
    ref.cast (ref null 17)
```

### Hypothesis
Local 0 has ref type `module 31` (likely HashMap/Vec struct) and the code tries to move it into local 2 whose
declared type is `module 7` (String array). The two `$type` IDs differ even though the source likely intends
the same `String` or aggregate type. This is a GC type identity / canonicalization problem.

## stdlib_trait/buf_read.ark

- **Function index**: `16` (`_start`)
- **Offset**: `0xf76`
- **Function header**: `(func (;16;) (type 45))`

### Validation error
```
error: func 16 failed to validate

Caused by:
    0: type mismatch: expected i64, found (ref null $type) (at offset 0xf76)
```

- **Call target index**: `6` (`(import "wasi:cli/exit@0.2.0" "exit" (func (;6;) (type 4)))`)
- **Call target type**: `(type (;4;) (func (param i32)))`

### Dump context
```
  0xf70 | 10 06       | call function_index:6
  0xf72 | 00          | unreachable
  0xf73 | 0b          | end
  0xf74 | 20 01       | local_get local_index:1
  0xf76 | 21 58       | local_set local_index:88
  0xf78 | fb 17 11    | ref_cast_nullable hty:Concrete(Module(17))
  0xf7b | fb 02 11 00 | struct_get struct_type_index:17 field_index:0
  0xf7f | 20 58       | local_get local_index:88
  0xf81 | a7          | i32_wrap_i64
```

### Fixture source (first 50 lines)
```ark
// Issue #693: BufRead trait for buffered reading.
//
// Verifies that the BufRead trait's fill_buf returns remaining bytes
// without advancing the cursor, and that repeated calls are consistent.
use std::host::stdio
use std::io

fn main() {
    // Create a memory buffer and write "ABC" (65, 66, 67)
    let buf = io::new_memory_buffer()
    let data: Vec<i32> = Vec_new_i32()
    push(data, 65)  // 'A'
    push(data, 66)  // 'B'
    push(data, 67)  // 'C'

    let written: i32 = io::write_bytes(buf, data)
    assert(written == 3)

    // fill_buf should return all 3 bytes without advancing cursor
    let filled: Vec<i32> = io::fill_buffer(buf)
    assert(len(filled) == 3)
    assert(get_unchecked(filled, 0) == 65)
    assert(get_unchecked(filled, 1) == 66)
    assert(get_unchecked(filled, 2) == 67)

    // Cursor should still be at 0 (fill_buf does not consume)
    let pos: i64 = io::position(buf)
    assert(pos == 0)

    // Read 1 byte to advance cursor
    let one: Vec<i32> = io::read_bytes(buf, 1)
    assert(len(one) == 1)
    assert(get_unchecked(one, 0) == 65)

    // fill_buf should now return remaining 2 bytes
    let filled2: Vec<i32> = io::fill_buffer(buf)
    assert(len(filled2) == 2)
    assert(get_unchecked(filled2, 0) == 66)
    assert(get_unchecked(filled2, 1) == 67)

    // Cursor should be at 1
    let pos2: i64 = io::position(buf)
    assert(pos2 == 1)

    stdio::println("OK")
}
```

### WASM function start
```wat
(func (;16;) (type 45)
    (local (ref null 17) (ref null 17) (ref null 17) (ref null 17) (ref null 17) i64 i64 i64 (ref null 17) i64 i64 i64 ( ...
    call 12
    local.set 0
    local.get 0
    local.set 1
    i32.const 8
    array.new_default 8
    ref.cast (ref null 8)
    local.set 108
    local.get 108
    ref.cast (ref null 8)
```

### Hypothesis
Local 88 is declared as an i64 scalar, but the code stores a GC ref (`local.get 1`, type `ref null 17`). The
lowering still treats an enum/Option/Result or `String` payload as an i64 linear-memory value while the
surrounding code expects a GC reference.

## stdlib_trait/hash_trait.ark

- **Function index**: `14` (`_start`)
- **Offset**: `0xad6`
- **Function header**: `(func (;14;) (type 44))`

### Validation error
```
error: func 14 failed to validate

Caused by:
    0: type mismatch: expected anyref but nothing on stack (at offset 0xad6)
```

### Dump context
```
 0xad0 | 20 11       | local_get local_index:17
 0xad2 | 52          | i64_ne
 0xad3 | ad          | i64_extend_i32_u
 0xad4 | 21 1d       | local_set local_index:29
 0xad6 | fb 17 11    | ref_cast_nullable hty:Concrete(Module(17))
 0xad9 | fb 02 11 00 | struct_get struct_type_index:17 field_index:0
 0xadd | 20 1d       | local_get local_index:29
 0xadf | a7          | i32_wrap_i64
 0xae0 | fb 0b 08    | array_get array_type_index:8
```

### Fixture source (first 50 lines)
```ark
use std::host::stdio

/// User-defined Hash trait demonstrating the trait-based hashing protocol.
/// This mirrors the `Hash` trait defined in `std::core::hash`.
trait Hash {
    fn hash(self: Hash) -> i32
}

struct Point {
    x: i32,
    y: i32,
}

impl Hash for Point {
    fn hash(self: Point) -> i32 {
        let mut h = 37
        h = h * 31 + self.x
        if h < 0 {
            h = 0 - h
        }
        h = h * 31 + self.y
        if h < 0 {
            h = 0 - h
        }
        h
    }
}

/// Generic function with Hash trait bound (pass-through).
fn get_hash<T: Hash>(value: T) -> T {
    value
}

fn main() {
    let p1 = Point { x: 1, y: 2 }
    let p2 = Point { x: 1, y: 2 }
    let p3 = Point { x: 3, y: 4 }

    // Direct trait method call
    let h1 = hash(p1)
    let h2 = hash(p2)
    let h3 = hash(p3)
    stdio::println((h1 == h2).to_string())
    stdio::println((h1 != h3).to_string())

    // Generic function with Hash bound
    let p = get_hash(p1)
    let h4 = hash(p)
    stdio::println((h1 == h4).to_string())
}
```

### WASM function start
```wat
(func (;14;) (type 44)
    (local (ref null 10) i64 i64 (ref null 10) (ref null 10) i64 i64 (ref null 10) (ref null 10) i64 i64 (ref null 10) i ...
    struct.new_default 10
    ref.cast (ref null 10)
    local.set 0
    i64.const 1
    local.set 1
    local.get 0
    ref.cast (ref null 10)
    local.tee 0
    local.get 1
    i32.wrap_i64
```

### Hypothesis
After `local.set 29` the stack is empty, yet `ref.cast` expects a ref. The MIR sequence forgot to push the
source value before casting, or confused `local.set` with `local.tee`. This is a body-emission ordering bug.

## stdlib_trait/io_copy.ark

- **Function index**: `17` (`_start`)
- **Offset**: `0x1335`
- **Function header**: `(func (;17;) (type 46))`

### Validation error
```
error: func 17 failed to validate

Caused by:
    0: type mismatch: expected (ref null $type), found (ref null $type) (at offset 0x1335)
```

- **Call target index**: `16` (`(func (;16;) (type 45))`)
- **Call target type**: `(type (;45;) (func (param (ref null 9) (ref null 9)) (result i64)))`

### Dump context
```
 0x132d | 20 3f       | local_get local_index:63
 0x132f | 21 41       | local_set local_index:65
 0x1331 | 20 40       | local_get local_index:64
 0x1333 | 20 41       | local_get local_index:65
 0x1335 | 10 10       | call function_index:16
 0x1337 | 21 42       | local_set local_index:66
 0x1339 | 20 42       | local_get local_index:66
 0x133b | 21 43       | local_set local_index:67
 0x133d | 20 43       | local_get local_index:67
```

### Fixture source (first 50 lines)
```ark
// Issue #693: io::copy generic helper via Read/Write traits.
//
// Verifies that the trait-based copy moves bytes between two different
// memory buffers through Read and Write trait dispatch.
use std::host::stdio
use std::io

fn main() {
    // Source buffer: write "Hello World" then reset cursor to 0
    let src = io::new_memory_buffer()
    let data: Vec<i32> = Vec_new_i32()
    push(data, 72)   // 'H'
    push(data, 101)  // 'e'
    push(data, 108)  // 'l'
    push(data, 108)  // 'l'
    push(data, 111)  // 'o'
    push(data, 32)   // ' '
    push(data, 87)   // 'W'
    push(data, 111)  // 'o'
    push(data, 114)  // 'r'
    push(data, 108)  // 'l'
    push(data, 100)  // 'd'

    let written: i32 = io::write_bytes(src, data)
    assert(written == 11)

    // Reset cursor to 0 so we can read from the beginning
    let seek_result: i64 = io::seek_to(src, 0)
    assert(seek_result == 0)

    // Destination buffer
    let dst = io::new_memory_buffer()

    // Copy all bytes from src to dst via trait dispatch
    let copied: i32 = io::copy_trait(src, dst)
    assert(copied == 11)

    // Verify dst contains the copied bytes
    let dst_len: i64 = io::buffer_len(dst)
    assert(dst_len == 11)

    // Read back from dst to verify content
    let _seek2: i64 = io::seek_to(dst, 0)
    let out: Vec<i32> = io::read_bytes(dst, 11)
    assert(len(out) == 11)
    assert(get_unchecked(out, 0) == 72)   // 'H'
    assert(get_unchecked(out, 5) == 32)   // ' '
    assert(get_unchecked(out, 10) == 100) // 'd'

    stdio::println("OK")
```

### WASM function start
```wat
(func (;17;) (type 46)
    (local (ref null 17) (ref null 17) (ref null 17) (ref null 17) (ref null 17) i64 i64 i64 (ref null 17) i64 i64 i64 ( ...
    call 12
    local.set 0
    local.get 0
    local.set 1
    i32.const 8
    array.new_default 8
    ref.cast (ref null 8)
    local.set 127
    local.get 127
    ref.cast (ref null 8)
```

### Hypothesis
Function 16 is called with two GC refs whose concrete type IDs disagree (`expected (ref null $type), found
(ref null $type)`). The caller and callee were compiled with different type entries for the same
`String`/`Vec`/`Option` aggregate.

## stdlib_trait/ord_sort_by.ark

- **Function index**: `10` (`_start`)
- **Offset**: `0x523`
- **Function header**: `(func (;10;) (type 40))`

### Validation error
```
error: func 10 failed to validate

Caused by:
    0: type mismatch: expected i64, found (ref null $type) (at offset 0x523)
```

### Dump context
```
 0x51a | fb 05 11 01 | struct_set struct_type_index:17 field_index:1
 0x51e | 20 39       | local_get local_index:57
 0x520 | 1a          | drop
 0x521 | 20 01       | local_get local_index:1
 0x523 | 21 23       | local_set local_index:35
 0x525 | fb 17 11    | ref_cast_nullable hty:Concrete(Module(17))
 0x528 | fb 02 11 00 | struct_get struct_type_index:17 field_index:0
 0x52c | 20 23       | local_get local_index:35
 0x52e | a7          | i32_wrap_i64
```

### Fixture source (first 50 lines)
```ark
use std::host::stdio
use std::seq

fn main() {
    let v = Vec_new_i32()
    push(v, 3)
    push(v, 1)
    push(v, 2)
    let s = seq::sort_by(v)
    assert(get_unchecked(s, 0) == 1)
    assert(get_unchecked(s, 1) == 2)
    assert(get_unchecked(s, 2) == 3)
    stdio::println("OK")
}
```

### WASM function start
```wat
(func (;10;) (type 40)
    (local (ref null 17) (ref null 17) (ref null 17) i64 i64 i64 (ref null 17) i64 i64 i64 (ref null 17) i64 i64 i64 i64 ...
    i32.const 8
    array.new_default 8
    ref.cast (ref null 8)
    local.set 55
    local.get 55
    ref.cast (ref null 8)
    i32.const 0
    struct.new 17
    ref.cast (ref null 17)
    local.set 0
```

### Hypothesis
Local 35 is an i64 scalar but is assigned a GC ref (`local.get 1`). Similar to `buf_read`, this is a
scalar-vs-ref width mismatch for a trait method payload or return value.

## stdlib_wit/wit_type_basic.ark

- **Function index**: `11`
- **Offset**: `0x7ca`
- **Function header**: `(func (;11;) (type 44))`

### Validation error
```
error: func 11 failed to validate

Caused by:
    0: type mismatch: expected (ref null $type), found (ref $type) (at offset 0x7ca)
```

### Dump context
```
  0x7c2 | 21 41       | local_set local_index:65
  0x7c4 | 20 41       | local_get local_index:65
  0x7c6 | a7          | i32_wrap_i64
  0x7c7 | fb 07 07    | array_new_default array_type_index:7
  0x7ca | 21 3e       | local_set local_index:62
  0x7cc | 42 00       | i64_const value:0
  0x7ce | 21 42       | local_set local_index:66
  0x7d0 | 02 40       | block blockty:Empty
  0x7d2 | 03 40       | loop blockty:Empty
```

### Fixture source (first 50 lines)
```ark
use std::host::stdio
use std::wit::types

fn main() {
    let list_ty = types::wit_type_list(types::wit_type_string())
    stdio::println(types::wit_type_name(list_ty))
    stdio::println((types::wit_type_id(list_ty)).to_string())
    let opt_ty = types::wit_type_option(types::wit_type_u32())
    stdio::println(types::wit_type_name(opt_ty))
}
// compound WitType list/option
```

### WASM function start
```wat
(func (;11;) (type 44) (param i64) (result (ref null 7))
    (local i64 (ref null 17) (ref null 17) i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 (ref null 17) i64 i64 ...
    i64.const 12
    call 10
    local.set 2
    local.get 2
    local.set 3
    local.get 0
    local.set 4
    local.get 4
    i64.const 0
    i64.gt_s
```

### Hypothesis
`array.new_default 7` returns a non-null `(ref 7)`, but local 62 is declared `(ref null 7)`. The backend
should either declare the local non-null or insert an explicit nullability cast.

## stdlib_toml/toml_full_inline_dotted.ark

- **Function index**: `13`
- **Offset**: `0xa51`
- **Function header**: `(func (;13;) (type 50))`

### Validation error
```
error: func 13 failed to validate

Caused by:
    0: type mismatch: expected (ref null $type), found (ref $type) (at offset 0xa51)
```

### Dump context
```
  0xa49 | 04 40       | if blockty:Empty
  0xa4b | 42 00       | i64_const value:0
  0xa4d | a7          | i32_wrap_i64
  0xa4e | fb 07 07    | array_new_default array_type_index:7
  0xa51 | 21 0c       | local_set local_index:12
  0xa53 | 20 0c       | local_get local_index:12
  0xa55 | fb 17 29    | ref_cast_nullable hty:Concrete(Module(41))
  0xa58 | 0f          | return
  0xa59 | 0b          | end
```

### Fixture source (first 50 lines)
```ark
// Issue #705: Full TOML 1.0 inline tables and dotted keys.
use std::host::stdio
use std::toml

fn main() {
    // Inline table
    let input1 = String_from("point = { x = 1, y = 2 }")
    match toml::toml_parse(input1) {
        Result::Ok(table) => {
            match toml::toml_get(table, String_from("point")) {
                Option::Some(pt) => {
                    match toml::toml_get(pt, String_from("x")) {
                        Option::Some(x) => {
                            match toml::toml_as_int(x) {
                                Option::Some(n) => assert(n == 1),
                                None => assert(false),
                            }
                        },
                        None => assert(false),
                    }
                    match toml::toml_get(pt, String_from("y")) {
                        Option::Some(y) => {
                            match toml::toml_as_int(y) {
                                Option::Some(n) => assert(n == 2),
                                None => assert(false),
                            }
                        },
                        None => assert(false),
                    }
                },
                None => assert(false),
            }
        },
        Result::Err(_) => assert(false),
    }

    // Dotted keys
    let input2 = String_from("a.b.c = 42")
    match toml::toml_parse(input2) {
        Result::Ok(table) => {
            match toml::toml_get(table, String_from("a")) {
                Option::Some(a) => {
                    match toml::toml_get(a, String_from("b")) {
                        Option::Some(b) => {
                            match toml::toml_get(b, String_from("c")) {
                                Option::Some(c) => {
                                    match toml::toml_as_int(c) {
                                        Option::Some(n) => assert(n == 42),
                                        None => assert(false),
                                    }
```

### WASM function start
```wat
(func (;13;) (type 50) (param (ref null 7) i64 i64) (result (ref null 7))
    (local i64 i64 i64 i64 i64 i64 i64 i64 i64 (ref null 41) i64 (ref null 17) (ref null 17) (ref null 17) (ref null 7)  ...
    local.get 0
    array.len
    i64.extend_i32_u
    local.set 3
    local.get 3
    local.set 4
    local.get 1
    i64.const 0
    i64.lt_s
    i64.extend_i32_u
```

### Hypothesis
`array.new_default 7` returns `(ref 7)`, and the code immediately casts it to `(ref null 41)`. The local is
declared as `ref null 41`, so the array type 7 and the target type 41 should be the same aggregate but the
emitter produced two different type IDs.

## stdlib_toml/toml_full_table_header.ark

- **Function index**: `12`
- **Offset**: `0x937`
- **Function header**: `(func (;12;) (type 49))`

### Validation error
```
error: func 12 failed to validate

Caused by:
    0: type mismatch: expected (ref null $type), found (ref $type) (at offset 0x937)
```

### Dump context
```
  0x92f | 01 6b       | 1 locals of type Ref(structref)
  0x931 | 42 00       | i64_const value:0
  0x933 | a7          | i32_wrap_i64
  0x934 | fb 07 07    | array_new_default array_type_index:7
  0x937 | 21 00       | local_set local_index:0
  0x939 | 20 00       | local_get local_index:0
  0x93b | fb 17 29    | ref_cast_nullable hty:Concrete(Module(41))
  0x93e | 0f          | return
  0x93f | 0b          | end
```

### Fixture source (first 50 lines)
```ark
// Issue #705: Full TOML 1.0 table header parsing.
use std::host::stdio
use std::toml

fn main() {
    // Table header + key=value entries
    let input = String_from("[server]\nhost = \"localhost\"\nport = 8080")
    match toml::toml_parse(input) {
        Result::Ok(table) => {
            // toml_get on nested table
            match toml::toml_get(table, String_from("server")) {
                Option::Some(server) => {
                    match toml::toml_get(server, String_from("host")) {
                        Option::Some(host) => {
                            match toml::toml_as_string(host) {
                                Option::Some(s) => assert(eq(s, String_from("localhost"))),
                                None => assert(false),
                            }
                        },
                        None => assert(false),
                    }
                    match toml::toml_get(server, String_from("port")) {
                        Option::Some(port) => {
                            match toml::toml_as_int(port) {
                                Option::Some(n) => assert(n == 8080),
                                None => assert(false),
                            }
                        },
                        None => assert(false),
                    }
                },
                None => assert(false),
            }
        },
        Result::Err(e) => {
            stdio::println(concat(String_from("parse error: "), e))
            assert(false)
        },
    }

    // find_toml_section public API
    let section = toml::find_toml_section(input, String_from("server"))
    assert(eq(section, String_from("host = \"localhost\"\nport = 8080")))

    // find_toml_value public API
    let val = toml::find_toml_value(input, String_from("port"))
    assert(eq(val, String_from("8080")))

    stdio::println(String_from("OK"))
}
```

### WASM function start
```wat
(func (;12;) (type 49) (result (ref null 7))
    (local (ref null 41) i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 f64 f64 f64 i64 (ref null 7) (ref null 7) (ref  ...
    i64.const 0
    i32.wrap_i64
    array.new_default 7
    local.set 0
    local.get 0
    ref.cast (ref null 41)
    return
  )
```

### Hypothesis
Identical to `toml_full_inline_dotted`: an `array.new_default 7` result is stored in a `(ref null 41)` local
and then `ref.cast` to `ref null 41`. Type 7 and type 41 mismatch for what should be a single array/struct
type.

## stdlib_host/host_module_contract.ark

- **Function index**: `17` (`_start`)
- **Offset**: `0xf44`
- **Function header**: `(func (;17;) (type 45))`

### Validation error
```
error: func 17 failed to validate

Caused by:
    0: type mismatch: expected i32, found i64 (at offset 0xf44)
```

- **Call target index**: `1` (`(import "wasi:cli/environment@0.2.0" "args-sizes" (func (;1;) (type 1)))`)
- **Call target type**: `(type (;1;) (func (param i32 i32) (result i32)))`

### Dump context
```
  0xf3c | 1a          | drop
  0xf3d | 0b          | end
  0xf3e | 42 cc 00    | i64_const value:76
  0xf41 | 42 d0 00    | i64_const value:80
  0xf44 | 10 01       | call function_index:1
  0xf46 | 1a          | drop
  0xf47 | 42 cc 00    | i64_const value:76
  0xf4a | 28 02 00    | i32_load memarg:MemArg { align: 2, max_align: 2, offset: 0, memory: 0 }
  0xf4d | ad          | i64_extend_i32_u
```

### Fixture source (first 50 lines)
```ark
// Fixture: host_module_contract.ark
// Verifies the std::host module contracts:
//   - clock: monotonic_now and now_ms return non-negative values
//   - env: args and var are available
//   - process: exit/abort are callable (no direct test of side effects)
use std::host::clock
use std::host::env
use std::host::stdio

fn main() {
    // ── Host clock contract ──
    let t_ns = clock::monotonic_now()
    assert(t_ns >= 0)
    let t_ms = clock::now_ms()
    assert(t_ms >= 0)
    if t_ns > 0 {
        stdio::println("clock:non-zero-ns")
    } else {
        stdio::println("clock:zero-ns")
    }
    if t_ms > 0 {
        stdio::println("clock:non-zero-ms")
    } else {
        stdio::println("clock:zero-ms")
    }

    // ── Host env contract ──
    let ac = env::arg_count()
    assert(ac >= 0)
    stdio::println(concat("env:args:", (ac).to_string()))

    // var may return None on targets without environ support
    match env::var("PATH") {
        Option::Some(_) => stdio::println("env:path-set"),
        None => stdio::println("env:path-unset"),
    }
}
```

### WASM function start
```wat
(func (;17;) (type 45)
    (local i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 (ref null 7) (ref null 7) i64 i64 i64 i64 (ref null 7 ...
    call 14
    local.set 0
    local.get 0
    local.set 1
    local.get 1
    i64.const 0
    i32.wrap_i64
    i64.extend_i32_s
    i64.ge_s
    i64.extend_i32_u
```

### Hypothesis
A WASI P2 core import with i32 pointer parameters (`args-sizes`) is called with i64 Memory64 addresses. The
lowering must convert i64 addresses to i32 at host import call sites, or emit an adapter module.

## stdlib_json/json_perf_decode.ark

- **Function index**: `35` (`_start`)
- **Offset**: `0x3509`
- **Function header**: `(func (;35;) (type 59))`

### Validation error
```
error: func 35 failed to validate

Caused by:
    0: type mismatch: expected (ref null $type), found i64 (at offset 0x3509)
```

- **Call target index**: `33` (`(func (;33;) (type 55))`)
- **Call target type**: `(type (;55;) (func (param (ref null 21)) (result (ref null 21))))`

### Dump context
```
 0x3500 | fb 02 25 01 | struct_get struct_type_index:37 field_index:1
 0x3504 | ad          | i64_extend_i32_u
 0x3505 | 21 06       | local_set local_index:6
 0x3507 | 20 06       | local_get local_index:6
 0x3509 | 10 21       | call function_index:33
 0x350b | 21 07       | local_set local_index:7
 0x350d | 20 07       | local_get local_index:7
 0x350f | fb 02 15 00 | struct_get struct_type_index:21 field_index:0
 0x3513 | ad          | i64_extend_i32_u
```

### Fixture source (first 50 lines)
```ark
use std::host::stdio
use std::json

fn main() {
    // Stress-test json_decode_string via json_as_string with escape sequences
    match json::parse("\"hello\\nworld\\ttabbed\"") {
        Result::Ok(v) => {
            match json::json_as_string(v) {
                Option::Some(s) => {
                    stdio::println((len(s)).to_string())
                    stdio::println(s)
                },
                None => stdio::println("not string"),
            }
        },
        Result::Err(e) => stdio::println(concat("error:", json::parse_error_message(e))),
    }
    // Long string with no escapes (tests plain-text batching)
    let long = "\"abcdefghijklmnopqrstuvwxyz0123456789\""
    match json::parse(long) {
        Result::Ok(v) => {
            match json::json_as_string(v) {
                Option::Some(s) => stdio::println((len(s)).to_string()),
                None => stdio::println("not string"),
            }
        },
        Result::Err(e) => stdio::println(concat("error:", json::parse_error_message(e))),
    }
}
```

### WASM function start
```wat
(func (;35;) (type 59)
    (local (ref null 7) (ref null 21) i64 i64 i64 i64 i64 (ref null 21) i64 i64 i64 i64 (ref null 7) i64 (ref null 7) (r ...
    i64.const 22
    i32.wrap_i64
    array.new_default 7
    local.set 0
    i64.const 0
    local.set 47
    block ;; label = @1
      loop ;; label = @2
        local.get 47
        i64.const 22
```

### Hypothesis
Function 33 expects a GC reference (`ref null 21`) but receives an i64 produced by `struct.get` field 1 of
type 37 followed by `i64.extend_i32_u`. This i64 is a linear-memory enum/Option/Result payload that has not
been boxed into the GC type `JsonValue`.

# Cross-cutting root causes

## 1. Memory64 i64 addresses vs WASI P2 i32 pointer imports

`host_module_contract` is the clearest example. The core module uses Memory64
(`(memory i64 1)`), so all linear-memory addresses are `i64`. WASI Preview 2 core
imports (`wasi:cli/environment::args-sizes`, `wasi:cli/stdout::write`, etc.) are
defined with `i32` pointer parameters. The selfhost emitter currently passes `i64`
values directly to these imports. Options:

- Truncate i64 addresses to i32 at every host import call site.
- Generate a P2 adapter core module that accepts i64 and re-exports
i32-compatible functions.
- Treat host import parameters as `i64` only when the target memory is Memory64
and let the component/adapter layer handle width conversion.

## 2. Enum / Option / Result still in linear memory

`docs/current-state.md` states that enums, Options, and Results use a
discriminated union in linear memory. In Memory64 this payload is read as `i64`.
Functions that expect GC references (`json_as_string__1`, `String::fmt_debug`, etc.)
receive these `i64` payloads and fail validation. This is the ADR-035 GC layout
migration gap.

## 3. GC type identity / nullability mismatches

`hashmap_generic_demo`, `io_copy`, `wit_type_basic`, and the two `toml_*` fixtures
show that the same aggregate type gets multiple Wasm GC type IDs (`module 7` vs
`module 41`, `module 31` vs `module 7`, etc.). This breaks `ref.cast`, `local.set`,
and function calls. Likely causes:

- Type layout cache does not canonicalize struct/array types across modules.
- `array.new_default` returns non-null `(ref T)` while the local is declared
`(ref null T)` without an explicit cast.
- Separate lowering paths for `String`, array, and struct produce distinct GC type
entries for semantically identical types.

## 4. MIR/backend lowering stack bugs

`hash_trait` has an empty stack before `ref.cast` (`local.set` consumed the value
and nothing reloaded it). This is a body-emission ordering bug in the MIR lowering
for `String` hashing or `fmt_debug` dispatch.

## Relevant source files

- `src/compiler/mir/lower/call_emit.ark` — call-site emission, arg/return lowering
- `src/compiler/mir/lower/call_types.ark` / `call_rewrite.ark` — callee type selection
- `src/compiler/mir/lower/type_info_to_mir_value.ark` — MirValueType / GC ref handling
- `src/compiler/mir/lower/signature_registry_*.ark` — host import signature registration
- `src/compiler/wasm/` — GC type section, struct/array emit, `ref.cast`/`struct.get`/`array.*`
- `src/compiler/component/wasi_p2_stub.ark` — WASI P2 stub core modules
- `std/collections/hash_map.ark` / `std/collections/hash.ark` — HashMap / Hash trait wrappers
- `std/text/json/*.ark` — JSON enum/value types
- `std/wit/` and `std/toml/` — WIT/TOML aggregate types

## Recommended design questions

1. How should Memory64 i64 linear addresses be lowered at host/WASI import boundaries?
2. When will enum / Option / Result be migrated from linear-memory i64 payloads to
GC references?
3. How are GC struct/array type IDs canonicalized across modules and between nullable
vs non-null variants?
4. What is the contract for `array.new_default` / `struct.new` result nullability in the
current GC backend?
5. Which of these 10 fixtures can be fixed independently, and which must wait for
ADR-035?