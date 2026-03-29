# Self-Hosting Stdlib Checklist

Tracks every stdlib function the Arukellt compiler needs to self-host.
Cross-referenced against `std/manifest.toml`, `std/prelude.ark`, and the
Rust compiler crates (`ark-lexer`, `ark-parser`, `ark-resolve`,
`ark-typecheck`, `ark-mir`, `ark-wasm`).

**Legend**

| Mark | Meaning |
|------|---------|
| ✅ | Implemented (in prelude or importable module) |
| ⚠️ | Exists but with limitations (wrong signature, only i32 variant, etc.) |
| ❌ | Missing — must be implemented before v5 Phase 1 |

---

## Required for Compiler Implementation

### String Operations

| Function | Signature | Status | Used By | Notes |
|----------|-----------|--------|---------|-------|
| `char_at` | `fn char_at(s: String, i: i32) -> i32` | ✅ | Lexer | Prelude |
| `substring` | `fn substring(s: String, start: i32, end: i32) -> String` | ✅ | Lexer, Parser | Prelude |
| `slice` | `fn slice(s: String, start: i32, end: i32) -> String` | ✅ | Lexer | Prelude |
| `contains` | `fn contains(s: String, sub: String) -> bool` | ✅ | Lexer, Resolve | Prelude |
| `starts_with` | `fn starts_with(s: String, prefix: String) -> bool` | ✅ | Lexer, Parser | Prelude |
| `ends_with` | `fn ends_with(s: String, suffix: String) -> bool` | ✅ | Parser | Prelude |
| `split` | `fn split(s: String, delim: String) -> Vec<String>` | ✅ | Parser, Resolve | Prelude |
| `trim` | `fn trim(s: String) -> String` | ✅ | Lexer | Prelude |
| `trim_start` | `fn trim_start(s: String) -> String` | ✅ | Lexer | `std::text` |
| `trim_end` | `fn trim_end(s: String) -> String` | ✅ | Lexer | `std::text` |
| `concat` | `fn concat(a: String, b: String) -> String` | ✅ | All | Prelude |
| `eq` | `fn eq(a: String, b: String) -> bool` | ✅ | All | Prelude |
| `clone` | `fn clone(s: String) -> String` | ✅ | Resolve, Typecheck | Prelude |
| `replace` | `fn replace(s: String, from: String, to: String) -> String` | ✅ | Diagnostics | Prelude |
| `join` | `fn join(v: Vec<String>, sep: String) -> String` | ✅ | Resolve, Diagnostics | Prelude |
| `String_new` | `fn String_new() -> String` | ✅ | All | Prelude |
| `String_from` | `fn String_from(s: String) -> String` | ✅ | All | Prelude |
| `push_char` | `fn push_char(s: String, c: char)` | ✅ | Lexer | Prelude |
| `to_lower` | `fn to_lower(s: String) -> String` | ✅ | Parser | Prelude |
| `to_upper` | `fn to_upper(s: String) -> String` | ✅ | Diagnostics | Prelude |
| `is_empty` | `fn is_empty(s: String) -> bool` | ✅ | Lexer, Parser | Builtin |
| `index_of` | `fn index_of(s: String, needle: String) -> i32` | ✅ | Lexer | `std::text` |
| `lines` | `fn lines(s: String) -> Vec<String>` | ✅ | Diagnostics | `std::text` |
| `chars` | `fn chars(s: String) -> Vec<String>` | ✅ | Lexer | `std::text` |
| `repeat` | `fn repeat(s: String, n: i32) -> String` | ✅ | Diagnostics | `std::text` |
| `pad_left` | `fn pad_left(s: String, width: i32, fill: String) -> String` | ✅ | Diagnostics | `std::text` |
| `pad_right` | `fn pad_right(s: String, width: i32, fill: String) -> String` | ✅ | Diagnostics | `std::text` |
| `len_bytes` | `fn len_bytes(s: String) -> i32` | ✅ | Lexer | `std::text` |
| `string_len` | `fn string_len(s: String) -> i32` | ❌ | Lexer | No prelude string length; must use `len_bytes` via import |
| `is_alphabetic` | `fn is_alphabetic(c: i32) -> bool` | ❌ | Lexer | Char classification needed for identifier scanning |
| `is_digit` | `fn is_digit(c: i32) -> bool` | ❌ | Lexer | Char classification needed for number scanning |
| `is_whitespace` | `fn is_whitespace(c: i32) -> bool` | ❌ | Lexer | Char classification needed for whitespace skipping |
| `is_alphanumeric` | `fn is_alphanumeric(c: i32) -> bool` | ❌ | Lexer | Needed for identifier continuation |
| `char_from_i32` | `fn char_from_i32(c: i32) -> char` | ❌ | Lexer | Convert codepoint back to char |
| `strip_prefix` | `fn strip_prefix(s: String, prefix: String) -> Option<String>` | ❌ | Parser | Rust lexer uses strip_prefix for literal parsing |
| `string_compare` | `fn string_compare(a: String, b: String) -> i32` | ❌ | Resolve, Wasm | Ordering comparison for deterministic output |

### Conversion Functions

| Function | Signature | Status | Used By | Notes |
|----------|-----------|--------|---------|-------|
| `i32_to_string` | `fn i32_to_string(n: i32) -> String` | ✅ | All | Prelude |
| `i64_to_string` | `fn i64_to_string(n: i64) -> String` | ✅ | Lexer | Prelude |
| `f64_to_string` | `fn f64_to_string(n: f64) -> String` | ✅ | Lexer | Prelude |
| `bool_to_string` | `fn bool_to_string(b: bool) -> String` | ✅ | Diagnostics | Prelude |
| `char_to_string` | `fn char_to_string(c: char) -> String` | ✅ | Lexer | Prelude |
| `to_string` | `fn to_string(x: any) -> String` | ✅ | All | Canonical user-facing conversion surface |
| `parse_i32` | `fn parse_i32(s: String) -> Result<i32, String>` | ✅ | Parser | Prelude |
| `parse_i64` | `fn parse_i64(s: String) -> Result<i64, String>` | ✅ | Lexer | Prelude |
| `parse_f64` | `fn parse_f64(s: String) -> Result<f64, String>` | ✅ | Lexer | Prelude |

### Vec Operations

| Function | Signature | Status | Used By | Notes |
|----------|-----------|--------|---------|-------|
| `push` | `fn push(v: Vec<T>, val: T)` | ✅ | All | Builtin, polymorphic |
| `pop` | `fn pop(v: Vec<T>) -> Option<T>` | ✅ | Parser (stack) | Builtin |
| `get` | `fn get(v: Vec<T>, i: i32) -> Option<T>` | ✅ | All | Builtin |
| `get_unchecked` | `fn get_unchecked(v: Vec<T>, i: i32) -> T` | ✅ | All | Builtin |
| `set` | `fn set(v: Vec<T>, i: i32, val: T)` | ✅ | All | Builtin |
| `len` | `fn len(v: Vec<T>) -> i32` | ✅ | All | Builtin |
| `clear` | `fn clear(v: Vec<T>)` | ✅ | Parser | Builtin |
| `Vec_new_i32` | `fn Vec_new_i32() -> Vec<i32>` | ✅ | Wasm | Prelude |
| `Vec_new_i64` | `fn Vec_new_i64() -> Vec<i64>` | ✅ | — | Prelude |
| `Vec_new_f64` | `fn Vec_new_f64() -> Vec<f64>` | ✅ | — | Prelude |
| `Vec_new_String` | `fn Vec_new_String() -> Vec<String>` | ✅ | Parser, Resolve | Prelude |
| `Vec_with_capacity_i32` | `fn Vec_with_capacity_i32(cap: i32) -> Vec<i32>` | ✅ | Wasm | Builtin |
| `Vec_with_capacity_String` | `fn Vec_with_capacity_String(cap: i32) -> Vec<String>` | ✅ | Parser | Builtin |
| `contains_i32` | `fn contains_i32(v: Vec<i32>, x: i32) -> bool` | ✅ | Wasm | Prelude |
| `contains_String` | `fn contains_String(v: Vec<String>, x: String) -> bool` | ✅ | Resolve | Prelude |
| `remove_i32` | `fn remove_i32(v: Vec<i32>, index: i32)` | ✅ | — | Prelude |
| `reverse_i32` | `fn reverse_i32(v: Vec<i32>)` | ✅ | — | Prelude |
| `reverse_String` | `fn reverse_String(v: Vec<String>)` | ✅ | — | Prelude |
| `sort_i32` | `fn sort_i32(v: Vec<i32>)` | ✅ | Wasm | Prelude |
| `sort_String` | `fn sort_String(v: Vec<String>)` | ✅ | Wasm | Prelude (deterministic output) |
| `map_i32_i32` | `fn map_i32_i32(v: Vec<i32>, f: fn(i32)->i32) -> Vec<i32>` | ✅ | MIR | Prelude |
| `map_String_String` | `fn map_String_String(v: Vec<String>, f: fn(String)->String) -> Vec<String>` | ✅ | Diagnostics | Prelude |
| `filter_i32` | `fn filter_i32(v: Vec<i32>, f: fn(i32)->bool) -> Vec<i32>` | ✅ | MIR | Prelude |
| `filter_String` | `fn filter_String(v: Vec<String>, f: fn(String)->bool) -> Vec<String>` | ✅ | Resolve | Prelude |
| `fold_i32_i32` | `fn fold_i32_i32(v: Vec<i32>, init: i32, f: fn(i32,i32)->i32) -> i32` | ✅ | MIR | Prelude |
| `any_i32` | `fn any_i32(v: Vec<i32>, f: fn(i32)->bool) -> bool` | ✅ | MIR | Prelude |
| `find_i32` | `fn find_i32(v: Vec<i32>, f: fn(i32)->bool) -> Option<i32>` | ✅ | MIR | Prelude |
| `remove_String` | `fn remove_String(v: Vec<String>, index: i32)` | ❌ | Resolve | Only `remove_i32` exists |
| `insert_at` | `fn insert_at(v: Vec<T>, index: i32, val: T)` | ❌ | Parser | Insert at arbitrary position |
| `last` | `fn last(v: Vec<T>) -> Option<T>` | ❌ | Parser | Get last element without pop |
| `index_of_String` | `fn index_of_String(v: Vec<String>, x: String) -> i32` | ❌ | Resolve | Find index of element |
| `enumerate` | `fn enumerate(v: Vec<T>) -> Vec<(i32, T)>` | ❌ | MIR, Wasm | Index-value pairs (needs tuples) |

### HashMap Operations

| Function | Signature | Status | Used By | Notes |
|----------|-----------|--------|---------|-------|
| `HashMap_i32_i32_new` | `fn HashMap_i32_i32_new() -> HashMap<i32, i32>` | ✅ | — | Builtin |
| `HashMap_i32_i32_insert` | `fn HashMap_i32_i32_insert(m: HashMap<i32,i32>, k: i32, v: i32)` | ✅ | — | Builtin |
| `HashMap_i32_i32_get` | `fn HashMap_i32_i32_get(m: HashMap<i32,i32>, k: i32) -> Option<i32>` | ✅ | — | Builtin |
| `HashMap_i32_i32_contains_key` | `fn HashMap_i32_i32_contains_key(m: HashMap<i32,i32>, k: i32) -> bool` | ✅ | — | Builtin |
| `HashMap_i32_i32_len` | `fn HashMap_i32_i32_len(m: HashMap<i32,i32>) -> i32` | ✅ | — | Builtin |
| `HashMap_String_i32_new` | `fn HashMap_String_i32_new() -> HashMap<String, i32>` | ❌ | Resolve, Typecheck | **Critical**: symbol tables use String keys |
| `HashMap_String_i32_insert` | `fn HashMap_String_i32_insert(m: HashMap<String,i32>, k: String, v: i32)` | ❌ | Resolve, Typecheck | |
| `HashMap_String_i32_get` | `fn HashMap_String_i32_get(m: HashMap<String,i32>, k: String) -> Option<i32>` | ❌ | Resolve, Typecheck | |
| `HashMap_String_i32_contains_key` | `fn HashMap_String_i32_contains_key(m: HashMap<String,i32>, k: String) -> bool` | ❌ | Resolve, Typecheck | |
| `HashMap_String_i32_remove` | `fn HashMap_String_i32_remove(m: HashMap<String,i32>, k: String) -> Option<i32>` | ❌ | Resolve | |
| `HashMap_String_i32_keys` | `fn HashMap_String_i32_keys(m: HashMap<String,i32>) -> Vec<String>` | ❌ | Resolve, Typecheck | Needed for iteration |
| `HashMap_String_i32_values` | `fn HashMap_String_i32_values(m: HashMap<String,i32>) -> Vec<i32>` | ❌ | Typecheck | |
| `HashMap_String_i32_len` | `fn HashMap_String_i32_len(m: HashMap<String,i32>) -> i32` | ❌ | Resolve | |
| `HashMap_String_String_new` | `fn HashMap_String_String_new() -> HashMap<String, String>` | ❌ | Typecheck | Type alias/display maps |
| `HashMap_String_String_insert` | `fn HashMap_String_String_insert(...)` | ❌ | Typecheck | |
| `HashMap_String_String_get` | `fn HashMap_String_String_get(...)` | ❌ | Typecheck | |
| `HashMap_i32_i32_remove` | `fn HashMap_i32_i32_remove(m: HashMap<i32,i32>, k: i32) -> Option<i32>` | ❌ | MIR | Missing remove for existing i32 map |
| `HashMap_i32_i32_keys` | `fn HashMap_i32_i32_keys(m: HashMap<i32,i32>) -> Vec<i32>` | ❌ | MIR | Missing iteration for existing i32 map |

### HashSet Operations

| Function | Signature | Status | Used By | Notes |
|----------|-----------|--------|---------|-------|
| `HashSet_i32_new` | `fn HashSet_i32_new() -> HashSet<i32>` | ❌ | MIR, Wasm | Deduplication of emitted items |
| `HashSet_i32_insert` | `fn HashSet_i32_insert(s: HashSet<i32>, v: i32) -> bool` | ❌ | MIR, Wasm | |
| `HashSet_i32_contains` | `fn HashSet_i32_contains(s: HashSet<i32>, v: i32) -> bool` | ❌ | MIR, Wasm | |
| `HashSet_i32_len` | `fn HashSet_i32_len(s: HashSet<i32>) -> i32` | ❌ | MIR | |
| `HashSet_String_new` | `fn HashSet_String_new() -> HashSet<String>` | ❌ | Typecheck, Wasm | String deduplication |
| `HashSet_String_insert` | `fn HashSet_String_insert(s: HashSet<String>, v: String) -> bool` | ❌ | Typecheck, Wasm | |
| `HashSet_String_contains` | `fn HashSet_String_contains(s: HashSet<String>, v: String) -> bool` | ❌ | Typecheck, Wasm | |

> **Note**: `std::collections::ordered::bitset_*` can serve as a partial
> substitute for `HashSet<i32>` when keys are small non-negative integers,
> but a general-purpose HashSet is still needed.

### Option Operations

| Function | Signature | Status | Used By | Notes |
|----------|-----------|--------|---------|-------|
| `unwrap` | `fn unwrap(o: Option<T>) -> T` | ✅ | All | Builtin |
| `unwrap_or` | `fn unwrap_or(o: Option<T>, default: T) -> T` | ✅ | All | Builtin |
| `unwrap_or_else` | `fn unwrap_or_else(o: Option<T>, f: fn()->T) -> T` | ⚠️ | Resolve | Declared but lacking FnSig in checker |
| `is_some` | `fn is_some(o: Option<T>) -> bool` | ✅ | All | Builtin |
| `is_none` | `fn is_none(o: Option<T>) -> bool` | ✅ | All | Builtin |
| `expect` | `fn expect(o: Option<T>, msg: String) -> T` | ✅ | Parser | Builtin |
| `ok_or` | `fn ok_or(o: Option<T>, err: E) -> Result<T, E>` | ✅ | Parser | Builtin |
| `ok` | `fn ok(r: Result<T, E>) -> Option<T>` | ✅ | — | Builtin |
| `map_option_i32_i32` | `fn map_option_i32_i32(o: Option<i32>, f: fn(i32)->i32) -> Option<i32>` | ✅ | — | Prelude |
| `map_option_String_String` | `fn map_option_String_String(o: Option<String>, f: fn(String)->String) -> Option<String>` | ✅ | — | Builtin |
| `map_option_i32_String` | `fn map_option_i32_String(o: Option<i32>, f: fn(i32)->String) -> Option<String>` | ❌ | Diagnostics | Cross-type Option map |
| `and_then` | `fn and_then(o: Option<T>, f: fn(T)->Option<T>) -> Option<T>` | ❌ | Resolve, Typecheck | Monadic chaining |

### Result Operations

| Function | Signature | Status | Used By | Notes |
|----------|-----------|--------|---------|-------|
| `is_ok` | `fn is_ok(r: Result<T, E>) -> bool` | ✅ | All | Builtin |
| `is_err` | `fn is_err(r: Result<T, E>) -> bool` | ✅ | All | Builtin |
| `err` | `fn err(r: Result<T, E>) -> Option<E>` | ✅ | Diagnostics | Builtin |
| `map_result_i32_i32` | `fn map_result_i32_i32(r: Result<i32,E>, f: fn(i32)->i32) -> Result<i32,E>` | ✅ | — | Builtin |
| `unwrap_result` | `fn unwrap(r: Result<T, E>) -> T` | ⚠️ | All | Shared name with Option unwrap; may cause ambiguity |
| `map_err` | `fn map_err(r: Result<T, E>, f: fn(E)->E2) -> Result<T, E2>` | ❌ | Diagnostics | Error transformation |
| `and_then_result` | `fn and_then(r: Result<T, E>, f: fn(T)->Result<U, E>) -> Result<U, E>` | ❌ | Parser, Resolve | Monadic chaining |
| `unwrap_or_result` | `fn unwrap_or(r: Result<T, E>, default: T) -> T` | ❌ | Parser | Fallback on error |

### Box Operations

| Function | Signature | Status | Used By | Notes |
|----------|-----------|--------|---------|-------|
| `Box_new` | `fn Box_new(val: T) -> Box<T>` | ✅ | Parser | Builtin; recursive AST nodes |
| `unbox` | `fn unbox(b: Box<T>) -> T` | ✅ | Typecheck | Builtin |

### I/O and Host Operations

| Function | Signature | Status | Used By | Notes |
|----------|-----------|--------|---------|-------|
| `read_to_string` | `fn read_to_string(path: String) -> Result<String, String>` | ✅ | Driver | `std::host::fs` |
| `write_string` | `fn write_string(path: String, contents: String) -> Result<(), String>` | ✅ | Driver | `std::host::fs` |
| `print` | `fn print(s: String)` | ✅ | Driver | `std::host::stdio` |
| `println` | `fn println(s: String)` | ✅ | Driver, Diagnostics | `std::host::stdio` |
| `eprintln` | `fn eprintln(s: String)` | ✅ | Diagnostics | `std::host::stdio` |
| `exit` | `fn exit(code: i32)` | ✅ | Driver | `std::host::process` |
| `abort` | `fn abort()` | ✅ | — | `std::host::process` |
| `args` | `fn args() -> Vec<String>` | ✅ | Driver | `std::host::env` |
| `arg_count` | `fn arg_count() -> i32` | ✅ | Driver | `std::host::env` |
| `arg_at` | `fn arg_at(index: i32) -> Option<String>` | ✅ | Driver | `std::host::env` |
| `has_flag` | `fn has_flag(name: String) -> bool` | ✅ | Driver | `std::host::env` |
| `var` | `fn var(name: String) -> Option<String>` | ✅ | Driver | `std::host::env` |

### Path Operations

| Function | Signature | Status | Used By | Notes |
|----------|-----------|--------|---------|-------|
| `path::join` | `fn join(base: String, rel: String) -> String` | ✅ | Driver | `std::path` |
| `path::parent` | `fn parent(path: String) -> String` | ✅ | Driver | `std::path` |
| `path::file_name` | `fn file_name(path: String) -> String` | ✅ | Driver | `std::path` |
| `path::extension` | `fn extension(path: String) -> String` | ✅ | Driver | `std::path` |
| `path::is_absolute` | `fn is_absolute(path: String) -> bool` | ✅ | Driver | `std::path` |

### Numeric / Bit Operations

| Function | Signature | Status | Used By | Notes |
|----------|-----------|--------|---------|-------|
| Arithmetic (`+`, `-`, `*`, `/`, `%`) | operators | ✅ | All | Language-level |
| Comparison (`<`, `>`, `<=`, `>=`, `==`, `!=`) | operators | ✅ | All | Language-level |
| Bitwise (`&`, `\|`, `^`, `<<`, `>>`) | operators | ✅ | Wasm | Language-level |
| `abs` | `fn abs(x: i32) -> i32` | ✅ | Diagnostics | Prelude |
| `min` | `fn min(a: i32, b: i32) -> i32` | ✅ | Parser | Prelude |
| `max` | `fn max(a: i32, b: i32) -> i32` | ✅ | Parser | Prelude |
| `clamp_i32` | `fn clamp_i32(x: i32, lo: i32, hi: i32) -> i32` | ✅ | — | Prelude |
| `i32_to_i64` | `fn i32_to_i64(x: i32) -> i64` | ❌ | Wasm | Widening cast for binary encoding |
| `i64_to_i32` | `fn i64_to_i32(x: i64) -> i32` | ❌ | Wasm | Narrowing cast |

### Byte Operations

| Function | Signature | Status | Used By | Notes |
|----------|-----------|--------|---------|-------|
| `bytes_new` | `fn bytes_new() -> Vec<i32>` | ✅ | Wasm | `std::bytes` |
| `bytes_push` | `fn bytes_push(b: Vec<i32>, byte: i32)` | ✅ | Wasm | `std::bytes` |
| `bytes_len` | `fn bytes_len(b: Vec<i32>) -> i32` | ✅ | Wasm | `std::bytes` |
| `bytes_get` | `fn bytes_get(b: Vec<i32>, index: i32) -> i32` | ✅ | Wasm | `std::bytes` |
| `bytes_eq` | `fn bytes_eq(a: Vec<i32>, b: Vec<i32>) -> bool` | ✅ | Wasm | `std::bytes` |
| `leb128_encode_u32` | `fn leb128_encode_u32(x: i32) -> Vec<i32>` | ✅ | Wasm | `std::bytes` |
| `leb128_encode_i32` | `fn leb128_encode_i32(x: i32) -> Vec<i32>` | ✅ | Wasm | `std::bytes` |
| `u32_to_le_bytes` | `fn u32_to_le_bytes(x: i32) -> Vec<i32>` | ✅ | Wasm | `std::bytes` |
| `u32_from_le_bytes` | `fn u32_from_le_bytes(b: Vec<i32>) -> i32` | ✅ | Wasm | `std::bytes` |
| `hex_encode` | `fn hex_encode(b: Vec<i32>) -> String` | ✅ | — | `std::bytes` |
| `hex_decode` | `fn hex_decode(s: String) -> Vec<i32>` | ✅ | — | `std::bytes` |
| `leb128_encode_i64` | `fn leb128_encode_i64(x: i64) -> Vec<i32>` | ❌ | Wasm | i64 globals/constants |
| `bytes_concat` | `fn bytes_concat(a: Vec<i32>, b: Vec<i32>) -> Vec<i32>` | ❌ | Wasm | Combine byte buffers |

### Sorting (Deterministic Output)

| Function | Signature | Status | Used By | Notes |
|----------|-----------|--------|---------|-------|
| `sort_i32` | `fn sort_i32(v: Vec<i32>)` | ✅ | Wasm | Prelude |
| `sort_String` | `fn sort_String(v: Vec<String>)` | ✅ | Wasm | Prelude; needed for deterministic emission |
| `sort_i64` | `fn sort_i64(v: Vec<i64>)` | ✅ | — | Prelude |
| `sort_f64` | `fn sort_f64(v: Vec<f64>)` | ✅ | — | Prelude |

### Control Flow and Error Handling

| Function | Signature | Status | Used By | Notes |
|----------|-----------|--------|---------|-------|
| `panic` | `fn panic(s: String)` | ✅ | All | Prelude |
| `assert` | `fn assert(cond: bool)` | ✅ | — | Prelude |
| `error_message` | `fn error_message(e: Error) -> String` | ✅ | Diagnostics | `std::core::error` |

### Compiler-Specific Collections

| Function | Signature | Status | Used By | Notes |
|----------|-----------|--------|---------|-------|
| `arena_new` | `fn arena_new() -> Vec<i32>` | ✅ | Parser, Typecheck | `std::collections::compiler` (experimental) |
| `arena_alloc` | `fn arena_alloc(a: Vec<i32>, val: i32) -> i32` | ✅ | Parser, Typecheck | `std::collections::compiler` |
| `arena_get` | `fn arena_get(a: Vec<i32>, idx: i32) -> i32` | ✅ | Parser, Typecheck | `std::collections::compiler` |
| `arena_len` | `fn arena_len(a: Vec<i32>) -> i32` | ✅ | Parser, Typecheck | `std::collections::compiler` |
| `hashmap_new` | `fn hashmap_new() -> Vec<i32>` | ⚠️ | Resolve | `std::collections::hash`; i32-only, Vec-encoded |
| `hashmap_set` | `fn hashmap_set(m: Vec<i32>, k: i32, v: i32)` | ⚠️ | Resolve | i32 keys only |
| `hashmap_get` | `fn hashmap_get(m: Vec<i32>, k: i32) -> i32` | ⚠️ | Resolve | Returns sentinel -1 on miss, not Option |
| `hashmap_contains` | `fn hashmap_contains(m: Vec<i32>, k: i32) -> bool` | ⚠️ | Resolve | i32 keys only |
| `hashmap_size` | `fn hashmap_size(m: Vec<i32>) -> i32` | ⚠️ | Resolve | i32 keys only |

---

## Summary of Missing Items (❌)

The following functions **must be implemented** before v5 Phase 1 (self-hosting)
can begin. They are grouped by implementation priority.

### Priority 1 — Blockers (no workaround)

| # | Function | Category | Reason |
|---|----------|----------|--------|
| 1 | `HashMap_String_i32_new` | HashMap | Symbol tables require String-keyed maps |
| 2 | `HashMap_String_i32_insert` | HashMap | |
| 3 | `HashMap_String_i32_get` | HashMap | |
| 4 | `HashMap_String_i32_contains_key` | HashMap | |
| 5 | `HashMap_String_i32_remove` | HashMap | Scope cleanup |
| 6 | `HashMap_String_i32_keys` | HashMap | Iteration over symbol names |
| 7 | `HashMap_String_i32_values` | HashMap | |
| 8 | `HashMap_String_i32_len` | HashMap | |
| 9 | `HashMap_String_String_new` | HashMap | Type display / alias maps |
| 10 | `HashMap_String_String_insert` | HashMap | |
| 11 | `HashMap_String_String_get` | HashMap | |
| 12 | `HashSet_String_new` | HashSet | Type deduplication in typechecker |
| 13 | `HashSet_String_insert` | HashSet | |
| 14 | `HashSet_String_contains` | HashSet | |
| 15 | `HashSet_i32_new` | HashSet | MIR/Wasm function deduplication |
| 16 | `HashSet_i32_insert` | HashSet | |
| 17 | `HashSet_i32_contains` | HashSet | |
| 18 | `HashSet_i32_len` | HashSet | |

### Priority 2 — Required (workaround possible but fragile)

| # | Function | Category | Reason |
|---|----------|----------|--------|
| 19 | `is_alphabetic` | String | Lexer character classification |
| 20 | `is_digit` | String | Lexer number scanning |
| 21 | `is_whitespace` | String | Lexer whitespace handling |
| 22 | `is_alphanumeric` | String | Lexer identifier continuation |
| 23 | `char_from_i32` | String | Codepoint-to-char conversion |
| 24 | `string_len` | String | Prelude-level string length (currently only `len_bytes` in std::text) |
| 25 | `and_then` (Option) | Option | Monadic chaining in resolver |
| 26 | `and_then_result` | Result | Monadic chaining in parser |
| 27 | `map_err` | Result | Error transformation in diagnostics |
| 28 | `HashMap_i32_i32_remove` | HashMap | Missing remove on existing map |
| 29 | `HashMap_i32_i32_keys` | HashMap | Missing iteration on existing map |

### Priority 3 — Desirable (can be deferred)

| # | Function | Category | Reason |
|---|----------|----------|--------|
| 30 | `strip_prefix` | String | Convenience; can use starts_with + substring |
| 31 | `string_compare` | String | Ordering; can encode manually |
| 32 | `remove_String` | Vec | Can implement in user code |
| 33 | `insert_at` | Vec | Can shift manually |
| 34 | `last` | Vec | Can use `get(v, len(v)-1)` |
| 35 | `index_of_String` | Vec | Linear scan in user code |
| 36 | `enumerate` | Vec | Manual index tracking |
| 37 | `map_option_i32_String` | Option | Cross-type map |
| 38 | `unwrap_or_result` | Result | Can use is_ok + unwrap |
| 39 | `leb128_encode_i64` | Bytes | i64 wasm encoding |
| 40 | `bytes_concat` | Bytes | Can push in loop |
| 41 | `i32_to_i64` | Numeric | Widening cast |
| 42 | `i64_to_i32` | Numeric | Narrowing cast |

---

## Coverage Summary

| Category | Implemented | Missing | Coverage |
|----------|-------------|---------|----------|
| String | 28 | 7 | 80% |
| Conversion | 8 | 0 | 100% |
| Vec | 27 | 5 | 84% |
| HashMap | 5 | 13 | 28% |
| HashSet | 0 | 7 | 0% |
| Option | 10 | 2 | 83% |
| Result | 4 | 4 | 50% |
| Box | 2 | 0 | 100% |
| I/O & Host | 12 | 0 | 100% |
| Path | 5 | 0 | 100% |
| Numeric | 7 | 2 | 78% |
| Bytes | 11 | 2 | 85% |
| Sorting | 4 | 0 | 100% |
| Control | 3 | 0 | 100% |
| **Total** | **126** | **42** | **75%** |

The biggest gap is **HashMap/HashSet with String keys** — the entire
name-resolution and type-checking pipeline depends on String-keyed maps,
and none exist yet. This is the single largest blocker for v5 self-hosting.
