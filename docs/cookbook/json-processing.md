# Cookbook: JSON Processing

Parse and stringify JSON primitive values using `std::json`.

> Policy: user-facing samples should avoid direct low-level prelude helper calls.
> Prefer module-local wrappers/facades; if needed, hide prelude details behind local helper functions.
>
> `std::json` is **Experimental** — full object/array support is planned for v4.

## Stringify Values

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::json

// Numbers
let age = json::JsonValue::int(30).to_text()
println(age) // "30"

// Booleans
let active = json::JsonValue::bool(true).to_text()
println(active) // "true"

// Strings (wraps in double quotes)
let name = json::JsonValue::string("Alice").to_text()
println(name) // "\"Alice\""

// Null
let empty = json::JsonValue::null_text()
println(empty) // "null"
```

## Parse Values

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::json

// Parse integer from JSON
let n = json::JsonValue::parse_i32("42")
assert_eq(n, 42)

// Parse negative integer
let neg = json::JsonValue::parse_i32("-7")
assert_eq(neg, 0 - 7)

// Parse boolean
let b = json::JsonValue::parse_bool("true")
assert(b)

let b2 = json::JsonValue::parse_bool("false")
assert(b2 == false)
```

## Inspect a Parsed Value

```ark
use std::json

match json::JsonValue::parse("{\"port\":8080}") {
    Result::Ok(root) => match root.get_field("port") {
        Option::Some(port) => match port.as_i32() {
            Option::Some(n) => println(n.to_string()),
            Option::None => println("port is not an integer"),
        },
        Option::None => println("port missing"),
    },
    Result::Err(e) => println(e.message()),
}
```

## Build a Simple JSON-like Output

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::json

// Manually build a key-value line for logging
fn json_field(key: String, value: String) -> String {
    concat(json::JsonValue::string(key).to_text(),
        concat(": ", value))
}

let line = concat(
    json_field("name", json::JsonValue::string("Alice").to_text()),
    concat(", ",
        json_field("age", json::JsonValue::int(30).to_text())))
println(line)
// "name": "Alice", "age": 30
```
