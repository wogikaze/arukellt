# std::json

**Stability**: Experimental
**Module**: `std::json`

## Overview

Minimal JSON stringify and parse for primitive values (string, number, bool, null).
Full object/array support is deferred to v4 (requires recursive data types).

> ⚠️ **Experimental**: API may change in minor versions.

## Functions

### `json_stringify_i32(val: i32) -> String`

Converts an integer to its JSON string representation.

**Example:**
```ark
use std::json

let s = json::json_stringify_i32(42)
println(s) // "42"
```

### `json_stringify_bool(val: bool) -> String`

Converts a boolean to `"true"` or `"false"`.

**Example:**
```ark
use std::json

let s = json::json_stringify_bool(true)
println(s) // "true"
```

### `json_stringify_string(val: String) -> String`

Wraps a string value in double quotes. Does not escape special characters.

**Example:**
```ark
use std::json

let s = json::json_stringify_string("hello")
println(s) // "\"hello\""
```

### `json_null() -> String`

Returns the JSON `"null"` literal.

**Example:**
```ark
use std::json

let s = json::json_null()
println(s) // "null"
```

### `json_parse_i32(s: String) -> i32`

Parses a JSON integer string into `i32`. Supports negative numbers. Does not validate input beyond digit parsing.

**Example:**
```ark
use std::json

let n = json::json_parse_i32("-42")
println(i32_to_string(n)) // "-42"
```

### `json_parse_bool(s: String) -> bool`

Returns `true` if the input string equals `"true"`, `false` otherwise.

**Example:**
```ark
use std::json

let b = json::json_parse_bool("true")
println(bool_to_string(b)) // "true"
```
