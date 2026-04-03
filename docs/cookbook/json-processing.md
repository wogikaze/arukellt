# Cookbook: JSON Processing

Parse and stringify JSON primitive values using `std::json`.

> `std::json` is **Experimental** — full object/array support is planned for v4.

## Stringify Values

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::json

// Numbers
let age = json::json_stringify_i32(30)
println(age) // "30"

// Booleans
let active = json::json_stringify_bool(true)
println(active) // "true"

// Strings (wraps in double quotes)
let name = json::json_stringify_string("Alice")
println(name) // "\"Alice\""

// Null
let empty = json::json_null()
println(empty) // "null"
```

## Parse Values

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::json

// Parse integer from JSON
let n = json::json_parse_i32("42")
assert_eq(n, 42)

// Parse negative integer
let neg = json::json_parse_i32("-7")
assert_eq(neg, 0 - 7)

// Parse boolean
let b = json::json_parse_bool("true")
assert(b)

let b2 = json::json_parse_bool("false")
assert(b2 == false)
```

## Build a Simple JSON-like Output

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::json

// Manually build a key-value line for logging
fn json_field(key: String, value: String) -> String {
    concat(json::json_stringify_string(key),
        concat(": ", value))
}

let line = concat(
    json_field("name", json::json_stringify_string("Alice")),
    concat(", ",
        json_field("age", json::json_stringify_i32(30))))
println(line)
// "name": "Alice", "age": 30
```
