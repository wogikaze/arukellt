# std::time

**Stability**: Stable
**Module**: `std::time`

## Overview

Monotonic clock access and duration calculation. `monotonic_now` returns nanoseconds since an unspecified epoch.
Values must not be compared across processes.

## Functions

### `monotonic_now() -> i64`

Returns the current monotonic clock value in nanoseconds. Backed by WASI `clock_time_get`.

**Example:**

```ark
use std::time

let start = time::monotonic_now()
// ... work ...
let end = time::monotonic_now()
println(i64_to_string(end - start))
```

### `elapsed_ms(start: i64) -> i64`

Returns milliseconds elapsed since `start` (obtained from `monotonic_now`).

**Example:**

```ark
use std::time

let start = time::monotonic_now()
// ... work ...
let ms = time::elapsed_ms(start)
println(concat("elapsed: ", concat(i64_to_string(ms), "ms")))
```

### `duration_ms(start: i64, end: i64) -> i64`

Returns the duration between two monotonic timestamps in milliseconds. Returns `0` if `end < start`.

**Example:**

```ark
use std::time

let t0 = time::monotonic_now()
// ... work ...
let t1 = time::monotonic_now()
let ms = time::duration_ms(t0, t1)
println(concat("took ", concat(i64_to_string(ms), "ms")))
```

### `duration_us(start: i64, end: i64) -> i64`

Returns the duration between two monotonic timestamps in microseconds. Returns `0` if `end < start`.

**Example:**

```ark
use std::time

let t0 = time::monotonic_now()
// ... work ...
let t1 = time::monotonic_now()
println(concat("took ", concat(i64_to_string(time::duration_us(t0, t1)), "µs")))
```

### `duration_ns(start: i64, end: i64) -> i64`

Returns the raw nanosecond difference. Returns `0` if `end < start`.

**Example:**

```ark
use std::time

let t0 = time::monotonic_now()
let t1 = time::monotonic_now()
println(concat("delta: ", concat(i64_to_string(time::duration_ns(t0, t1)), "ns")))
```
