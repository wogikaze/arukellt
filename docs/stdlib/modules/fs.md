# std::fs

**Stability**: Stable
**Module**: `std::fs`

## Overview

Filesystem operations backed by WASI P1 intrinsics. Provides reading and writing of text files.
Additional operations (exists, mkdir, etc.) require new WASI imports planned for v4.

## Functions

### `read_to_string(path: String) -> Result<String, String>`

Reads the entire contents of a file as a UTF-8 string. Returns `Err` if the file does not exist or cannot be read.

**Example:**
```ark
use std::fs

let result = fs::read_to_string("config.toml")
match result {
    Ok(contents) => println(contents),
    Err(e) => eprintln(concat("read error: ", e)),
}
```

### `write_string(path: String, contents: String) -> Result<String, String>`

Writes a string to a file, creating or overwriting it. Returns `Err` on failure.

**Example:**
```ark
use std::fs

let result = fs::write_string("output.txt", "hello world")
match result {
    Ok(_) => println("written"),
    Err(e) => eprintln(concat("write error: ", e)),
}
```

## Target Constraints

| API | T1 | WASI P1 | WASI P2 |
|-----|----|---------|---------| 
| `read_to_string` | ❌ | ✅ | ✅ |
| `write_string` | ❌ | ✅ | ✅ |

Calling these on target T1 produces a compile error (`E0093`).
