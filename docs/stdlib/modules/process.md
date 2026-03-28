# std::process

**Stability**: Stable
**Module**: `std::process`

## Overview

Process control: termination and (planned) command-line argument access.
Currently `exit` uses `panic` as a fallback since WASI `proc_exit` is not yet imported.

## Functions

### `exit(code: i32)`

Terminates the process with the given exit code. Code `0` is a clean exit (no-op, lets the program end naturally).
Non-zero codes trigger a panic with a descriptive message.

**Example:**
```ark
use std::process

process::exit(0) // clean exit
```

### `abort()`

Immediately aborts the process with a panic message.

**Example:**
```ark
use std::process

process::abort() // panics with "abort: process aborted"
```

## Planned API (v3)

The following functions are planned for v3 when full WASI args/env support is available:

```ark
pub fn args() -> Vec<String>
pub fn args_nth(i: i32) -> Option<String>
pub fn args_count() -> i32
pub fn env_var(name: String) -> Option<String>
```
