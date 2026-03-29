# Cookbook: Testing Patterns

Using `std::test` assert functions for effective tests.

## Basic Assertions

```ark
// assert — check a boolean condition
assert(1 + 1 == 2)
assert(len("hello") == 5)

// assert_eq — check equality (panics with both values on failure)
assert_eq(max(3, 5), 5)
assert_eq(min(3, 5), 3)
assert_eq(abs(0 - 7), 7)

// assert_ne — check inequality
assert_ne(1, 2)
```

## Type-Specific Assertions

```ark
// i64 equality
assert_eq_i64(1000000000_i64, 1000000000_i64)

// String equality
assert_eq_str("hello", "hello")
assert_eq_str(i32_to_string(42), "42")
```

## Testing Option Values

```ark
let some_val = Some(42)
let none_val: Option<i32> = None

assert(is_some(some_val))
assert(is_none(none_val))
assert_eq(unwrap(some_val), 42)
assert_eq(unwrap_or(none_val, 0), 0)
```

## Testing Result Values

```ark
let ok_val: Result<i32, String> = Ok(42)
let err_val: Result<i32, String> = Err("not found")

assert(is_ok(ok_val))
assert(is_err(err_val))
assert_eq(unwrap(ok_val), 42)
assert_eq_str(unwrap_err(err_val), "not found")
```

## Testing String Operations

```ark
use std::text

// Test string functions
assert_eq_str(text::to_uppercase("hello"), "HELLO")
assert_eq_str(text::trim("  hi  "), "hi")
assert(text::contains("hello world", "world"))
assert(text::starts_with("hello", "hel"))

// Test parse round-trip
let n = unwrap(parse_i32("42"))
assert_eq(n, 42)
assert_eq_str(i32_to_string(n), "42")
```

## Testing Collections

```ark
let v = Vec_new_i32()
push(v, 10)
push(v, 20)
push(v, 30)

assert_eq(len(v), 3)
assert_eq(get(v, 0), 10)
assert_eq(get(v, 2), 30)

// Test after sort
let unsorted = Vec_new_i32()
push(unsorted, 3)  push(unsorted, 1)  push(unsorted, 2)
sort_i32(unsorted)
assert_eq(get(unsorted, 0), 1)
assert_eq(get(unsorted, 1), 2)
assert_eq(get(unsorted, 2), 3)

// Test HashMap
let m = HashMap_new_String_i32()
hashmap_insert(m, "a", 1)
hashmap_insert(m, "b", 2)
assert_eq(hashmap_len(m), 2)
assert(hashmap_contains_key(m, "a"))
assert_eq(unwrap(hashmap_get(m, "b")), 2)
```

## Testing Path Operations

```ark
use std::path

assert_eq_str(path::join("/usr", "local"), "/usr/local")
assert_eq_str(path::file_name("/a/b/c.txt"), "c.txt")
assert_eq_str(path::extension("file.tar.gz"), "gz")
assert_eq_str(path::parent("/a/b/c"), "/a/b")
assert(path::is_absolute("/home"))
assert(path::is_absolute("relative") == false)
```

## Testing with Error Cases

```ark
// Verify that invalid input produces errors
let bad = parse_i32("abc")
assert(is_err(bad))

// Verify file-not-found
use std::host::fs
let result = fs::read_to_string("nonexistent.txt")
assert(is_err(result))
```

## Structuring Test Files

Tests are organized as fixture files under `tests/fixtures/`.
Each test file has a `main()` that runs assertions directly:

```ark
// tests/fixtures/test_path.ark
use std::path
use std::host::stdio

fn main() {
    assert_eq_str(path::join("a", "b"), "a/b")
    assert_eq_str(path::parent("a/b"), "a")
    assert_eq_str(path::extension("f.txt"), "txt")
    stdio::println("all path tests passed")
}
```
