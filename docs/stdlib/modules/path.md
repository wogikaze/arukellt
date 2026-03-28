# std::path

**Stability**: Stable
**Module**: `std::path`

## Overview

Pure string-based path manipulation. All paths use `/` as separator (POSIX/WASI convention).
Paths are represented as `String`; no dedicated `Path` type.

## Functions

### `join(base: String, child: String) -> String`

Joins two path segments with `/`. Handles trailing slashes on `base` and empty inputs.

**Example:**
```ark
use std::path

let p = path::join("/usr", "local")
println(p) // "/usr/local"
```

### `parent(p: String) -> String`

Returns the parent directory of the path. Returns `""` if there is no parent.

**Example:**
```ark
use std::path

let dir = path::parent("/usr/local/bin")
println(dir) // "/usr/local"
```

### `file_name(p: String) -> String`

Returns the final component of the path (after the last `/`).

**Example:**
```ark
use std::path

let name = path::file_name("/usr/local/readme.md")
println(name) // "readme.md"
```

### `extension(p: String) -> String`

Returns the file extension (after the last `.` in the file name). Returns `""` if none.

**Example:**
```ark
use std::path

let ext = path::extension("archive.tar.gz")
println(ext) // "gz"
```

### `with_extension(p: String, ext: String) -> String`

Replaces the file extension. If the path has no extension, appends one. Pass `""` to remove the extension.

**Example:**
```ark
use std::path

let p = path::with_extension("main.ark", "wasm")
println(p) // "main.wasm"
```

### `is_absolute(p: String) -> bool`

Returns `true` if the path starts with `/`.

**Example:**
```ark
use std::path

println(bool_to_string(path::is_absolute("/home")))  // "true"
println(bool_to_string(path::is_absolute("src/lib"))) // "false"
```

## Internal Helpers

### `last_index_of(s: String, needle: String) -> i32`

Finds the last occurrence of `needle` in `s`. Returns `-1` if not found.
Used internally by `parent`, `file_name`, `extension`, and `with_extension`.
